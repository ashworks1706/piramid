// this module seres as low level binary data structure


// the sizes are not explicitly stored because vector<i8> already knows the size, we just store the
// size so we know how many bytes to read back

use std::mem; // we need this to calculate byte sizes


pub struct Entry{
    pub key: Vec<u8>, // we store raw bytes, not strings -> binary keys
    pub value: Vec<u8>, 
    pub timestamp: u128,  // nano seconds
}

impl Entry{
    pub fn new(key: Vec<u8>, value: Vec<u8>, timestamp: u128) -> Entry{
        Entry{
            key,
            value,
            timestamp,
        }
    }

    // convert entry struct into array of bytes
    pub fn encode(&self) -> Vec<u8>{
        let mut out = Vec::new();

        let key_len = self.key.len() as u32;
        let val_len = self.value.len() as u32;

        // writing header -> 16 bytes timestamp, 4 bytes key size, 4 bytes value size

        // to_be_bytes -> It takes a number like 500 and breaks it down into raw bytes [0, 0, 1, 244].
        out.extend_from_slice(&self.timestamp.to_be_bytes());
        out.extend_from_slice(&key_len.to_be_bytes());
        out.extend_from_slice(&key_len.to_be_bytes());

        // now write it to payload

        out.extend_from_slice(&self.key)
        out.extend_from_slice(&self.key)

        out
    }
    
    // convert byteful data to original entry struct
    // we return option since the result can be corrupted as well
    pub fn decode(data: &[u8]) -> Result<Entry,&'static str>{
        // Result: This is a generic enum used for error handling in Rust's standard library. It has two possible variants:
        // Ok(T): The operation was successful, and it returned a value of type T. In this case, T is Entry (presumably a user-defined struct or enum).
        // Err(E): The operation failed, and it returned an error value of type E. In this case, E is &'static str.
        // Entry: This is the type returned in the success case (Ok variant). 
        // &'static str: This specifies the type of the error value returned in the failure case (Err variant).
        
        // timestamp (16) * Key size (4) + Value size (4) = 24 bytes

        let header_size = 24;

        if data.len() < header_size {
            return Err("Data too short to contain a header");
        }

        // we take first 16 bytes for timestamp 
        // we take bytes 16 <-> 20 bytes for keys 
        // we take bytes 20 <-> 24 bytes for values
        
        let timestamp_bytes : [u8; 16] = data[0..16].try_into().unwrap();
        let timestamp = u128::from_be_bytes(timestamp_bytes);

        let klen_bytes : [u8; 4] = data[16..20].try_into().unwrap();
        let klen = u32::from_be_bytes(klen_bytes) as usize;

        let vlen_bytes : [u8; 4] = data[20..24].try_into().unwrap();
        let vlen = u32::from_be_bytes(vlen_bytes) as usize;


        // we check if the size matches the payload requirements

        let total_size = header_size + klen + vlen;

        if data.len()< total_size{
            return Err("Data too short to contain declared in header");
        }

        // reading the payload
        let key_start = header_size;
        let key_end = key_start + klen;
        let val_end = key_end + vlen;

        let key = data[key_start..key_end].to_vec();
        let value = data[key_end..val_end].to_vec();

        Ok(Entry{
            key,
            value,
            timestamp,
        })


    }
}
















