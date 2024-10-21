use std::collections::HashMap;
use std::hash::Hash;
use std::ptr::copy_nonoverlapping;

use crate::collections::CollectionItem;
use crate::transport::MESS_SIZE;

// dict -----------------------------------------------------------------------

/*
DictMessage

common head:
|1B - type | 4B - u32 value id | 1B - update | = 6B

---------
dict all:
| 1B - dict type | 8B - u64 count | ... | 8B - u64 size |
data: | key | value | * count

empty:
| 1B - dict type | 8B - u64 count = 0 |

---------
dict set:
no data:
| 1B - dict type | key | value | ...

with data:
| 1B - dict type | ... | 8B - u64 size |
data: | key | value |

------------
dict remove:
no data:
| 1B - dict type | key |

with data:
| 1B - dict type | ... | 8B - u64 size |
data: | key | ...

*/

const DICT_ALL: u8 = 20;
const DICT_SET: u8 = 21;
const DICT_REMOVE: u8 = 22;

pub enum DictMessage<K, V> {
    All(HashMap<K, V>),
    Set(K, V),
    Remove(K),
}

pub trait WriteDictMessage: Send + Sync + 'static {
    fn write_message(&self, head: &mut [u8]) -> Option<Vec<u8>>;
}

impl<K, V> WriteDictMessage for DictMessage<K, V>
where
    K: CollectionItem,
    V: CollectionItem,
{
    fn write_message(&self, head: &mut [u8]) -> Option<Vec<u8>> {
        match self {
            DictMessage::All(dict) => {
                head[0] = DICT_ALL;

                let count = dict.len();
                head[1..9].copy_from_slice(&(count as u64).to_le_bytes());

                // empty dict
                if count == 0 {
                    return None;
                }
                // all static
                else if K::SIZE > 0 && V::SIZE > 0 {
                    let size = dict.len() * (K::SIZE + V::SIZE);
                    let mut data = vec![0; size];
                    for (i, (key, value)) in dict.iter().enumerate() {
                        key.write_static(data[i * (K::SIZE + V::SIZE)..].as_mut());
                        value.write_static(data[i * (K::SIZE + V::SIZE) + K::SIZE..].as_mut());
                    }
                    Some(data)
                }
                // all dynamic
                else if K::SIZE == 0 && V::SIZE == 0 {
                    let mut keys_sizes = vec![0u16; count];
                    let mut keys_data = Vec::new();
                    let mut values_sizes = vec![0u16; count];
                    let mut values_data = Vec::new();
                    for (i, (key, value)) in dict.iter().enumerate() {
                        let k_data = key.get_dynamic();
                        keys_sizes[i] = (k_data.len() as u16).to_le();
                        keys_data.extend_from_slice(&k_data);
                        let v_data = value.get_dynamic();
                        values_sizes[i] = (v_data.len() as u16).to_le();
                        values_data.extend_from_slice(&v_data);
                    }

                    let mut final_data =
                        vec![
                            0u8;
                            count * size_of::<u16>() * 2 + keys_data.len() + values_data.len()
                        ];

                    unsafe {
                        copy_nonoverlapping(
                            keys_sizes.as_ptr() as *const u8,
                            final_data.as_mut_ptr(),
                            count * size_of::<u16>(),
                        );
                        let mut position = count * size_of::<u16>();

                        copy_nonoverlapping(
                            values_sizes.as_ptr() as *const u8,
                            final_data[position..].as_mut_ptr(),
                            count * size_of::<u16>(),
                        );
                        position += count * size_of::<u16>();

                        copy_nonoverlapping(
                            keys_data.as_ptr(),
                            final_data[position..].as_mut_ptr(),
                            keys_data.len(),
                        );
                        position += keys_data.len();

                        copy_nonoverlapping(
                            values_data.as_ptr(),
                            final_data[position..].as_mut_ptr(),
                            values_data.len(),
                        );
                    }

                    Some(final_data)
                }
                // key dynamic
                else if K::SIZE == 0 {
                    let mut keys_sizes = vec![0u16; count];
                    let mut keys_data = Vec::new();
                    let mut values_data = vec![0u8; count * V::SIZE];
                    for (i, (key, value)) in dict.iter().enumerate() {
                        let k_data = key.get_dynamic();
                        keys_sizes[i] = (k_data.len() as u16).to_le();
                        keys_data.extend_from_slice(&k_data);
                        value.write_static(values_data[i * V::SIZE..].as_mut());
                    }

                    let mut final_data =
                        vec![0u8; count * size_of::<u16>() + keys_data.len() + values_data.len()];

                    unsafe {
                        copy_nonoverlapping(
                            keys_sizes.as_ptr() as *const u8,
                            final_data.as_mut_ptr(),
                            count * size_of::<u16>(),
                        );
                        let mut position = count * size_of::<u16>();

                        copy_nonoverlapping(
                            values_data.as_ptr(),
                            final_data[position..].as_mut_ptr(),
                            values_data.len(),
                        );
                        position += values_data.len();

                        copy_nonoverlapping(
                            keys_data.as_ptr(),
                            final_data[position..].as_mut_ptr(),
                            keys_data.len(),
                        );
                    }

                    Some(final_data)
                }
                // value dynamic
                else {
                    let mut keys_data = vec![0; count * K::SIZE];
                    let mut values_sizes = vec![0u16; count];
                    let mut values_data = Vec::new();
                    for (i, (key, value)) in dict.iter().enumerate() {
                        key.write_static(keys_data[i * K::SIZE..].as_mut());
                        let v_data = value.get_dynamic();
                        values_sizes[i] = (v_data.len() as u16).to_le();
                        values_data.extend_from_slice(&v_data);
                    }

                    let mut final_data =
                        vec![0u8; count * K::SIZE + values_sizes.len() + values_data.len()];

                    unsafe {
                        copy_nonoverlapping(
                            values_sizes.as_ptr() as *const u8,
                            final_data.as_mut_ptr(),
                            values_sizes.len(),
                        );
                        let mut position = values_sizes.len();

                        copy_nonoverlapping(
                            keys_data.as_ptr(),
                            final_data[position..].as_mut_ptr(),
                            keys_data.len(),
                        );

                        position += keys_data.len();
                        copy_nonoverlapping(
                            values_data.as_ptr(),
                            final_data[position..].as_mut_ptr(),
                            values_data.len(),
                        );
                    }

                    Some(final_data)
                }
            }

            DictMessage::Set(key, value) => {
                head[0] = DICT_SET;

                // all static
                if K::SIZE > 0 && V::SIZE > 0 {
                    let size = K::SIZE + V::SIZE;
                    if size < MESS_SIZE {
                        key.write_static(head[1..].as_mut());
                        value.write_static(head[1 + K::SIZE..].as_mut());
                        return None;
                    }

                    let mut data = vec![0; size];
                    key.write_static(data[0..].as_mut());
                    value.write_static(data[K::SIZE..].as_mut());
                    Some(data)
                // all dynamic
                } else if K::SIZE == 0 && V::SIZE == 0 {
                    let mut data = key.get_dynamic();
                    data.extend(value.get_dynamic());
                    Some(data)
                // key dynamic
                } else if K::SIZE == 0 {
                    let k_data = key.get_dynamic();
                    let size = k_data.len() + V::SIZE;
                    let mut data = vec![0; size];
                    data[0..k_data.len()].copy_from_slice(&k_data);
                    value.write_static(data[k_data.len()..].as_mut());
                    Some(data)
                // value dynamic
                } else {
                    let v_data = value.get_dynamic();
                    let size = K::SIZE + v_data.len();
                    let mut data = vec![0; size];
                    key.write_static(data[0..].as_mut());
                    data[K::SIZE..].copy_from_slice(&v_data);
                    Some(data)
                }
            }

            DictMessage::Remove(key) => {
                head[0] = DICT_REMOVE;

                if K::SIZE == 0 {
                    let data = key.get_dynamic();
                    Some(data)
                } else if K::SIZE < MESS_SIZE {
                    key.write_static(head[1..].as_mut());
                    return None;
                } else {
                    let mut data = vec![0; K::SIZE];
                    key.write_static(data[0..].as_mut());
                    Some(data)
                }
            }
        }
    }
}

impl<K, V> DictMessage<K, V>
where
    K: CollectionItem + Eq + Hash,
    V: CollectionItem,
{
    pub fn read_message(head: &[u8], data: Option<Vec<u8>>) -> Result<DictMessage<K, V>, String> {
        let subtype = head[0];
        match subtype {
            DICT_ALL => {
                let count = u64::from_le_bytes(head[1..9].try_into().unwrap()) as usize;

                // empty dict
                let dict = if count == 0 {
                    if data.is_some() {
                        return Err("Dict get data but should be empty.".to_string());
                    }
                    HashMap::new()
                } else {
                    let data = data.ok_or("Dict data is missing.".to_string())?;
                    let mut dict = HashMap::new();

                    // all static
                    if K::SIZE > 0 && V::SIZE > 0 {
                        let bouth_size = K::SIZE + V::SIZE;
                        if bouth_size * count != data.len() {
                            return Err("Dict data is corrupted.".to_string());
                        }

                        for i in 0..count {
                            let key = K::read_item(&data[i * bouth_size..]);
                            let value = V::read_item(&data[i * bouth_size + K::SIZE..]);
                            dict.insert(key, value);
                        }
                        dict
                    }
                    // all dynamic
                    else if K::SIZE == 0 && V::SIZE == 0 {
                        if (count * size_of::<u16>() * 2) > data.len() {
                            return Err("Dict data is corrupted.".to_string());
                        }

                        let mut keys_sizes = vec![0u16; count];
                        let mut values_sizes = vec![0u16; count];

                        unsafe {
                            copy_nonoverlapping(
                                data.as_ptr(),
                                keys_sizes.as_mut_ptr() as *mut u8,
                                count * size_of::<u16>(),
                            );
                            copy_nonoverlapping(
                                data[count * size_of::<u16>()..].as_ptr(),
                                values_sizes.as_mut_ptr() as *mut u8,
                                count * size_of::<u16>(),
                            );
                        }



                        for i in 0..count {
                            let key_size = u16::from_ne_bytes();
                        }









                        let mut keys_sizes = vec![0u16; count];
                        let mut keys_data = Vec::new();
                        let mut values_sizes = vec![0u16; count];
                        let mut values_data = Vec::new();

                        let mut position = 0;
                        for _ in 0..count {
                            let key_size =
                                u16::from_le_bytes([data[position], data[position + 1]]) as usize;
                            keys_sizes.push(key_size as u16);
                            keys_data
                                .extend_from_slice(&data[position + 2..position + 2 + key_size]);
                            position += 2 + key_size;

                            let value_size =
                                u16::from_le_bytes([data[position], data[position + 1]]) as usize;
                            values_sizes.push(value_size as u16);
                            values_data
                                .extend_from_slice(&data[position + 2..position + 2 + value_size]);
                            position += 2 + value_size;
                        }

                        let mut dict = HashMap::new();
                        for i in 0..count {
                            let key = K::read_item(&keys_data[i * keys_sizes[i] as usize..]);
                            let value = V::read_item(&values_data[i * values_sizes[i] as usize..]);
                            dict.insert(key, value);
                        }
                        dict
                    }
                    // key dynamic
                    else if K::SIZE == 0 {
                        let mut keys_sizes = vec![0u16; count];
                        let mut keys_data = Vec::new();
                        let mut values_data = Vec::new();

                        let mut position = 0;
                        for _ in 0..count {
                            let key_size = u16::from_le_bytes(
                                data[position..position + 2].try_into().unwrap(),
                            ) as usize;
                            keys_sizes.push(key_size as u16);
                            keys_data
                                .extend_from_slice(&data[position + 2..position + 2 + key_size]);
                            position += 2 + key_size;

                            let value = V::read_item(&data[position..position + V::SIZE]);
                            values_data.extend_from_slice(&value.get_dynamic());
                            position += V::SIZE;
                        }

                        let mut dict = HashMap::new();
                        for i in 0..count {
                            let key = K::read_item(&keys_data[i * keys_sizes[i] as usize..]);
                            let value = V::read_item(&values_data[i * V::SIZE..]);
                            dict.insert(key, value);
                        }
                        dict
                    }
                    // value dynamic
                    else {
                        let mut keys_data = Vec::new();
                        let mut values_sizes = vec![0u16; count];
                        let mut values_data = Vec::new();

                        let mut position = 0;
                        for _ in 0..count {
                            let value_size = u16::from_le_bytes(
                                data[position..position + 2].try_into().unwrap(),
                            ) as usize;
                            values_sizes.push(value_size as u16);
                            values_data
                                .extend_from_slice(&data[position + 2..position + 2 + value_size]);
                            position += 2 + value_size;

                            let key = K::read_item(&data[position..position + K::SIZE]);
                            keys_data.extend_from_slice(&key.get_dynamic());
                            position += K::SIZE;
                        }

                        let mut dict = HashMap::new();
                        for i in 0..count {
                            let key = K::read_item(&keys_data[i * K::SIZE..]);
                            let value = V::read_item(&values_data[i * values_sizes[i] as usize..]);
                            dict.insert(key, value);
                        }
                        dict
                    }
                };

                // let dict = if count > 0 {
                //     let data = data.ok_or("Dict data is missing.".to_string())?;

                //     let mut dict = HashMap::new();
                //     let bouth_size = K::SIZE + V::SIZE;

                //     if bouth_size * count != data.len() {
                //         return Err("Dict data is corrupted.".to_string());
                //     }

                //     for i in 0..count {
                //         let key = K::read(&data[i * bouth_size..]);
                //         let value = V::read(&data[i * bouth_size + K::SIZE..]);
                //         dict.insert(key, value);
                //     }
                //     dict
                // } else {
                //     if data.is_some() {
                //         return Err("Dict get data but should be empty.".to_string());
                //     }

                //     HashMap::new()
                // };

                Ok(DictMessage::All(dict))
            }

            DICT_SET => match data {
                Some(data) => {
                    if K::SIZE + V::SIZE != data.len() {
                        return Err("Dict data is corrupted.".to_string());
                    }

                    let key = K::read(&data[0..]);
                    let value = V::read(&data[K::SIZE..]);
                    Ok(DictMessage::Set(key, value))
                }
                None => {
                    if K::SIZE + V::SIZE + 1 > MESS_SIZE {
                        return Err("Dict set failed to parse.".to_string());
                    }

                    let key = K::read(&head[1..]);
                    let value = V::read(&head[1 + K::SIZE..]);
                    Ok(DictMessage::Set(key, value))
                }
            },

            DICT_REMOVE => match data {
                Some(data) => {
                    if K::SIZE != data.len() {
                        return Err("Dict data is corrupted.".to_string());
                    }

                    let key = K::read(&data[0..]);
                    return Ok(DictMessage::Remove(key));
                }
                None => {
                    if K::SIZE + 1 > MESS_SIZE {
                        return Err("Dict remove failed to parse.".to_string());
                    }

                    let key = K::read(&head[1..]);
                    return Ok(DictMessage::Remove(key));
                }
            },

            _ => Err(format!("Unknown type of the dict message: {}", subtype,)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::HEAD_SIZE;

    #[test]
    fn test_dict_all_message() {
        let mut head = [0u8; HEAD_SIZE];
        let mut dict = HashMap::<i64, i64>::new();
        dict.insert(1, 2);
        dict.insert(3, 4);
        dict.insert(5, 6);
        dict.insert(7, 8);

        let message = DictMessage::All(dict.clone());

        let data = message.write_message(&mut head[6..]);
        assert!(data.is_some());
        let message = DictMessage::<i64, i64>::read_message(&mut head[6..], data).unwrap();

        match message {
            DictMessage::All(new_dict) => {
                assert_eq!(dict, new_dict);
            }
            _ => panic!("Wrong message type."),
        }
    }

    #[test]
    fn test_dict_all_empty() {
        let mut head = [0u8; HEAD_SIZE];
        let dict = HashMap::<i64, i64>::new();
        let message = DictMessage::All(dict.clone());

        let data = message.write_message(&mut head[6..]);
        assert!(data.is_none());
        let message = DictMessage::<i64, i64>::read_message(&mut head[6..], data).unwrap();

        match message {
            DictMessage::All(new_dict) => {
                assert_eq!(dict, new_dict);
            }
            _ => panic!("Wrong message type."),
        }
    }

    #[test]
    fn test_dict_set_message() {
        let mut head = [0u8; HEAD_SIZE];
        let key = 123456789u64;
        let value = 987654321u64;

        let message = DictMessage::Set(key, value);

        let data = message.write_message(&mut head[6..]);
        assert!(data.is_none());
        let message = DictMessage::<u64, u64>::read_message(&mut head[6..], data).unwrap();

        match message {
            DictMessage::Set(new_key, new_value) => {
                assert_eq!(key, new_key);
                assert_eq!(value, new_value);
            }
            _ => panic!("Wrong message type."),
        }
    }

    #[test]
    fn test_dict_remove_message() {
        let mut head = [0u8; HEAD_SIZE];
        let key = 123456789u64;

        let message = DictMessage::<u64, u64>::Remove(key);
        let data = message.write_message(&mut head[6..]);
        assert!(data.is_none());
        let message = DictMessage::<u64, u64>::read_message(&mut head[6..], data).unwrap();

        match message {
            DictMessage::Remove(new_key) => {
                assert_eq!(key, new_key);
            }
            _ => panic!("Wrong message type."),
        }
    }
}
