use std::collections::{HashMap, VecDeque};
use std::string::ToString;
use std::{fs, io::Write};

// enums -----------------------------------------------------------------------
pub fn parse_enum(enum_path: impl ToString, output_path: impl ToString) -> Result<(), String> {
    let mut lines: VecDeque<String> = fs::read_to_string(enum_path.to_string())
        .map_err(|e| format!("Failed to read file: {}", e))?
        .lines()
        .map(String::from)
        .collect();

    let mut file = fs::File::create(output_path.to_string())
        .map_err(|e| format!("Failed to create file: {}", e))?;

    file.write_all(b"# Ganerated by build.rs, do not edit\n")
        .unwrap();
    file.write_all(b"# ruff: noqa: D101\n").unwrap();
    file.write_all(b"from enum import Enum\n").unwrap();

    while lines.len() > 0 {
        let line = lines.pop_front().unwrap();

        if line.contains("pub enum") || line.contains("pub(crate) enum") {
            file.write_all(b"\n\n").unwrap();
            let enum_name = line.split(" ").collect::<Vec<&str>>()[2];
            file.write_all(format!("class {}(Enum):\n", enum_name).as_bytes())
                .unwrap();

            let mut counter = 0;
            loop {
                let line = lines.pop_front().unwrap();

                if line.contains("#") {
                    continue;
                } else if line.contains("}") {
                    break;
                } else {
                    let line = line.replace(",", "").trim().to_string();
                    if line.contains("=") {
                        file.write_all(format!("    {}\n", line).as_bytes())
                            .unwrap();
                    } else {
                        file.write_all(format!("    {} = {}\n", line, counter).as_bytes())
                            .unwrap();
                        counter += 1;
                    }
                }
            }
        }
    }

    file.flush().unwrap();
    Ok(())
}

// custem types ----------------------------------------------------------------
pub fn parse_custom_types(
    custom_types_path: impl ToString,
    output_path: impl ToString,
) -> Result<(), String> {
    let lines: Vec<String> = fs::read_to_string(custom_types_path.to_string())
        .map_err(|e| format!("Failed to read file: {}", e))?
        .lines()
        .map(String::from)
        .collect();

    let mut to_write: Vec<String> = Vec::new();

    fn parse_types(lines: &[String], to_write: &mut Vec<String>) {
        let line = lines[0].clone();

        if line.contains("//") {
            if line.contains("class") {
                let text = line.replace("//", "").trim().to_string();
                to_write.push(format!("\n{}\n", text));
                let mut i = 1;
                loop {
                    if lines[i].contains("//") {
                        let text = lines[i].replace("//", "").trim().to_string();
                        to_write.push(format!("    {}\n", text));
                        i += 1;
                    } else {
                        break;
                    }
                }
            } else {
                let text = line.replace("//", "").trim().to_string();
                to_write.push(format!("\n{}\n", text));
            }
        }
    }

    for (i, line) in lines.iter().enumerate() {
        if line.contains("#[derive") && !line.contains("//") {
            parse_types(&lines[i + 1..], &mut to_write);
        }
    }

    let mut file = fs::File::create(output_path.to_string())
        .map_err(|e| format!("Failed to create file: {}", e))?;

    file.write_all(b"# Ganerated by build.rs, do not edit\n")
        .unwrap();
    file.write_all(b"# ruff: noqa: UP013 F403 F405 D101 E302 E305\n")
        .unwrap();
    file.write_all(b"from typing import *  # type: ignore\n")
        .unwrap();
    file.write_all(b"from collections.abc import *  # type: ignore\n\n")
        .unwrap();

    for line in to_write {
        file.write_all(line.as_bytes()).unwrap();
    }

    file.flush().unwrap();
    Ok(())
}

// states -----------------------------------------------------------------------
struct Value {
    name: String,
}

enum Item {
    Value(String, Value),
    State(String, State),
}

struct State {
    name: String,
    items: Vec<Item>,
}

#[inline]
fn test_if_value(line: &str) -> bool {
    line.contains("Arc<Value<")
        || line.contains("Arc<ValueStatic<")
        || line.contains("Arc<ValueImage<")
        || line.contains("Arc<ValueEnum<")
        || line.contains("Arc<Signal<")
        || line.contains("Arc<ValueDict<")
        || line.contains("Arc<ValueList<")
}

impl State {
    fn new(name: String, lines: &Vec<String>) -> Result<Self, String> {
        let mut values = HashMap::new();
        let mut substates = HashMap::new();

        let mut started = false;
        let mut finished = false;
        for line in lines {
            if line.contains(format!("struct {}", name).as_str()) {
                started = true;
                continue;
            }

            if started {
                if line.contains("}") {
                    finished = true;
                    break;
                } else if line.contains("{") {
                    continue;
                } else if line.trim().is_empty() {
                    continue;
                } else if test_if_value(line) {
                    let item_name = line.split(": ").collect::<Vec<&str>>()[0];
                    let item_name = item_name
                        .split(" ")
                        .collect::<Vec<&str>>()
                        .last()
                        .unwrap()
                        .to_string();
                    let item =
                        line.split(": ").collect::<Vec<&str>>()[1][..line.len() - 1].to_string();
                    values.insert(item_name, item);
                } else {
                    let item_name = line.split(": ").collect::<Vec<&str>>()[0];
                    let item_name = item_name
                        .split(" ")
                        .collect::<Vec<&str>>()
                        .last()
                        .unwrap()
                        .to_string();
                    let item =
                        line.split(": ").collect::<Vec<&str>>()[1][..line.len() - 1].to_string();

                    let state = State::new(item, lines);
                    if let Ok(state) = state {
                        substates.insert(item_name, state);
                    }
                }
            }
        }

        if !finished {
            return Err(format!("Failed to parse state: {}", name));
        }

        let mut items = Vec::new();
        let mut started = false;
        let mut finished = false;

        for line in lines {
            if line.contains(format!("impl {}", name).as_str()) {
                started = true;
                continue;
            }

            if started {
                
            }
        }

        Ok(Self { name, items })
    }
}

// states for server -----------------------------------------------------------
pub fn parse_states_for_server(
    states_file: impl ToString,
    output_file: impl ToString,
    root_state: &'static str,
    imports: Vec<&'static str>,
) -> Result<(), String> {
    let lines: Vec<String> = fs::read_to_string(states_file.to_string())
        .map_err(|e| format!("Failed to read file: {}", e))?
        .lines()
        .map(String::from)
        .collect();

    Ok(())
}

// states -----------------------------------------------------------------------
fn type_map() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();
    map.insert("u8", "int");
    map.insert("u16", "int");
    map.insert("u32", "int");
    map.insert("u64", "int");
    map.insert("u128", "int");
    map.insert("usize", "int");
    map.insert("i8", "int");
    map.insert("i16", "int");
    map.insert("i32", "int");
    map.insert("i64", "int");
    map.insert("i128", "int");
    map.insert("isize", "int");
    map.insert("f32", "float");
    map.insert("f64", "float");
    map.insert("bool", "bool");
    map.insert("String", "str");
    map
}

fn parse_types(value: &str, custom: &Option<(String, String)>) -> Result<String, String> {
    let map = type_map();

    if let Some(v) = map.get(value) {
        return Ok(v.to_string());
    }

    if value == "()" {
        return Ok("".to_string());
    }

    if let Some((origin, python)) = custom {
        let origin = format!("{}::", origin);
        let python = format!("{}.", python);
        if value.contains(&origin) {
            return Ok(value.replace(&origin, &python));
        }
    }

    if value.starts_with("[") && value.ends_with("]") {
        let val = value[1..value.len() - 1].to_string();
        if val.contains(";") {
            let typ_val = val.split(";").collect::<Vec<&str>>()[0].trim();
            let nums = val.split(";").collect::<Vec<&str>>()[1].trim();

            let typ_val = parse_types(typ_val, custom)?;
            // let text =
        } else {
        }
    }

    Err(format!("Unknown type: {}", value))
}
