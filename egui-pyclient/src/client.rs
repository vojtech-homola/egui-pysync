use std::io::Write;
use std::net::{SocketAddrV4, TcpStream};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use egui::Context;
use egui_pysync::transport::{read_message, write_message, ReadMessage, WriteMessage};
use egui_pysync::{commands::CommandMessage, transport::HEAD_SIZE};

use crate::client_state::{ConnectionState, UIState};
use crate::states_creator::{ValuesCreator, ValuesList};

fn handle_message(
    message: ReadMessage,
    vals: &ValuesList,
    ui_state: &UIState,
) -> Result<(), String> {
    if let ReadMessage::Command(ref command) = message {
        match command {
            CommandMessage::Update(t) => {
                ui_state.update(*t);
            }
            _ => {}
        }
        return Ok(());
    }

    let update = match message {
        ReadMessage::Value(id, updata, head, data) => match vals.values.get(&id) {
            Some(value) => {
                value.update_value(head, data)?;
                updata
            }
            None => return Err(format!("Value with id {} not found", id)),
        },

        ReadMessage::Static(id, updata, head, data) => match vals.static_values.get(&id) {
            Some(value) => {
                value.update_value(head, data)?;
                updata
            }
            None => return Err(format!("Static with id {} not found", id)),
        },

        ReadMessage::Image(id, updata, image) => match vals.images.get(&id) {
            Some(value) => {
                value.update_image(image)?;
                updata
            }
            None => return Err(format!("Image with id {} not found", id)),
        },

        ReadMessage::Dict(id, updata, head, data) => match vals.dicts.get(&id) {
            Some(value) => {
                value.update_dict(head, data)?;
                updata
            }
            None => return Err(format!("Dict with id {} not found", id)),
        },

        ReadMessage::List(id, updata, head, data) => match vals.lists.get(&id) {
            Some(value) => {
                value.update_list(head, data)?;
                updata
            }
            None => return Err(format!("List with id {} not found", id)),
        },

        ReadMessage::Graph(id, updata, head, data) => match vals.graphs.get(&id) {
            Some(value) => {
                value.update_graph(head, data)?;
                updata
            }
            None => return Err(format!("Graph with id {} not found", id)),
        },

        ReadMessage::Signal(_, _, _) => {
            return Err("Signal message should not be handled in the client".to_string());
        }

        ReadMessage::Command(_) => unreachable!("should not parse Command message"),
    };

    if update {
        ui_state.update(0.);
    }

    Ok(())
}

fn start_gui_client(
    addr: SocketAddrV4,
    vals: ValuesList,
    version: u64,
    mut rx: Receiver<WriteMessage>,
    channel: Sender<WriteMessage>,
    ui_state: UIState,
    handshake: u64,
) {
    let client_thread = thread::Builder::new().name("Client".to_string());
    let _ = client_thread.spawn(move || loop {
        // wait for the connection signal
        ui_state.wait_connection();
        ui_state.set_state(ConnectionState::NotConnected);

        // try to connect to the server
        let res = TcpStream::connect(addr);
        if res.is_err() {
            continue;
        }

        // get the stream
        let mut stream_write = res.unwrap();
        let mut stream_read = stream_write.try_clone().unwrap();

        // clean mesage queue before starting
        for _v in rx.try_iter() {}

        // read thread -----------------------------------------
        let th_vals = vals.clone();
        let th_ui_state = ui_state.clone();
        let th_channel = channel.clone();

        let read_thread = thread::Builder::new().name("Read".to_string());
        let recv_tread = read_thread
            .spawn(move || {
                let mut head = [0u8; HEAD_SIZE];
                loop {
                    // read the message
                    let res = read_message(&mut head, &mut stream_read);
                    if let Err(e) = res {
                        println!("Error reading message: {:?}", e); // TODO: log error
                        break;
                    }
                    let (type_, data) = res.unwrap();

                    // parse message
                    let res = ReadMessage::parse(&head, type_, data);
                    if let Err(res) = res {
                        let error = format!("Error parsing message: {:?}", res);
                        th_channel
                            .send(WriteMessage::Command(CommandMessage::Error(error)))
                            .unwrap();
                        break;
                    }
                    let message = res.unwrap();

                    // handle the message
                    let res = handle_message(message, &th_vals, &th_ui_state);
                    if let Err(e) = res {
                        let error = format!("Error handling message: {:?}", e);
                        th_channel
                            .send(WriteMessage::Command(CommandMessage::Error(error)))
                            .unwrap();
                        break;
                    }
                }
            })
            .unwrap();

        // send thread -----------------------------------------
        let write_thread = thread::Builder::new().name("Write".to_string());
        let send_thread = write_thread
            .spawn(move || {
                // preallocate buffer
                let mut head = [0u8; HEAD_SIZE];

                // send handshake
                let handshake = CommandMessage::Handshake(version, handshake);
                let data = WriteMessage::Command(handshake).parse(&mut head);
                let res = write_message(&mut head, data, &mut stream_write);
                if let Err(e) = res {
                    println!("Error for sending hadnskae: {:?}", e); // TODO: log error
                    return rx;
                }

                loop {
                    // wait for the message from the channel
                    let message = rx.recv().unwrap();

                    // check if the message is terminate
                    if let WriteMessage::Terminate = message {
                        stream_write.flush().unwrap();
                        break;
                    }

                    // parse the message
                    let data = message.parse(&mut head);

                    // write the message
                    let res = write_message(&head, data, &mut stream_write);
                    if let Err(e) = res {
                        println!("Error for sending message: {:?}", e); // TODO: log error
                        break;
                    }
                }
                rx
            })
            .unwrap();

        ui_state.set_state(ConnectionState::Connected);

        // wait for the read thread to finish
        recv_tread.join().unwrap();

        // terminate the send thread
        channel.send(WriteMessage::Terminate).unwrap();
        rx = send_thread.join().unwrap();

        ui_state.set_state(ConnectionState::Disconnected);
    });
}

pub struct ClientBuilder {
    creator: ValuesCreator,
    channel: Sender<WriteMessage>,
    rx: Receiver<WriteMessage>,
}

impl ClientBuilder {
    pub fn new() -> Self {
        let (channel, rx) = std::sync::mpsc::channel();
        let creator = ValuesCreator::new(channel.clone());

        Self {
            creator,
            channel,
            rx,
        }
    }

    pub fn creator(&mut self) -> &mut ValuesCreator {
        &mut self.creator
    }

    pub fn build(self, context: Context, addr: [u8; 4], port: u16, handshake: u64) -> UIState {
        let Self {
            creator,
            channel,
            rx,
        } = self;

        let addr = SocketAddrV4::new(addr.into(), port);
        let (values, version) = creator.get_values();
        let ui_state = UIState::new(context, channel.clone());
        start_gui_client(
            addr,
            values,
            version,
            rx,
            channel,
            ui_state.clone(),
            handshake,
        );

        ui_state
    }
}
