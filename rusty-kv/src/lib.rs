use std::collections:Hashmap;
use std::fs::{File, OpenOptions}; // we use OpenOptions specificlly because we want to "edit" files
use std::io;
use std::path::Path;
use std::io::{self, SeekFrom, Read, Write, Seek, SeekFrom}; // for writing and seeking traits

pub mod entry;
use entry::Entry;

pub struct RustyKV{
    // the physical file on the disk
    file::File,

    // in memory map: key -> byte offset
    index: HashMap<String, u64>,
}


impl RustyKV{
    // we open hte file and prepare the database
    pub fn open(path : &str) -> io::Result<RustyKV> {
        let file = OpenOptions::new()
            .read(true) // setting config
            .write(true)
            .create(true)
            .append(true) // always write at the end
            .open(path)?;
        let mut store = RustyKV{
            file,
            index: HashMap::new();
        };

        store.load()?;

        // we will add core here t read the file and rebuild the index if database
        // exists already
        Ok(store)
    }

    // in log sturctured database, we will do this in four parts
    // 1-> wrap the input key value pair in the binary entry format like we are requesting
    // 2-> ask the operating system, where is the end of the file right now? basically the offset
    // 3-> append the bytes to the disk
    // 4-> update the hashmap in RAM

    pub fn set(&mut self, key: String, value: String) -> io::Result<()>{
        // preparing the data
        // using 0 for timestamps as demo

        let entry = Entry::new(
            key.clone().into_bytes(),
            // into_bytes converts string to vector of bytes
            // as_bytes converts string to slice of bytes
            // but we use into bytes because we need ownership of the data
            value.into_bytes(),
            0
            );

        let encoded = entry.encode();

        let size = encoded.len() as u64;

        // asking the OS where is the end of the file

        let current_offset= self.file.seek(SeekFrom::End(0))?;
        // when we open a file in append mode, the os usually handles writing at the end of the
        // file but we still need to know the offset where we are writing since we need to update
        // the in memory index


        // appending the data to the disk
        // we use write_all because write may not write all the bytes at once
        self.file.write_all(&encoded)?;

        // updating the in memory index
        // we map the key to the offset where the data is stored
        self.index.insert(key, current_offset);

        Ok(())

    }

    pub fn get(&mut self, key: String) -> io::Result<Option><String>>{
        // checking the index
        // ofcourse, if its not we return None option

        let offset = match self.index.get(&key){
            Some(&o) => o, // reference to the requested key we found 
            None => return Ok(None),
        }

        // once we find the offset, we move the file pointer to that offset (index) location
        self.file.seek(seekFrom::Start(offset))?; 

        // now once we're at the position we will read the header
        // temporaryirly create array to hold the data we will read off the file(first 24 bytes)
        let mut header = [0u8; 24];
        // 0u8 = 0 -> value is zero, u8 -> unsigned 8 bit integer        
        // we use this isntead of vec! because we don't know if klen + vlen is a variable until 
        // the program runs. which is why [] is on the heap memory

        // but why 24?
        // because we decided to store three specific piece of information as meta data at the start 
        // of every entry: timestamp, key_length and value length -> totalling to 24 bytes 

        // Byte Index:  0  1  ...  15 | 16 17 18 19 | 20 21 22 23
        //    [  TIMESTAMP  ] [  KEY LEN  ] [  VAL LEN  ]
        //    <---16 bytes--> <---4 bytes-> <---4 bytes->

        self.file.read_exact(&mut header)?;

        // now we take 16 to 20 to find out how long the key is  
        let klen_bytes : [u8; 4] = header[16..20].try_into().unwrap();
        let klen = u32::from_be_bytes(klen_bytes) as usize;

        // we take 20 to 24 bytes to find out how long the value is 
        let vlen_bytes: [u8; 4] = header[20..24].try_into().unwrap();
        let vlen = u32::from_be_bytes(vlen_bytes) as usize;

        // NOW we will create the payload which is basically a dynamic buffer exactly the size of
        // key + value
        let total_size = klen + vlen;
        let mut payload = vec![0u8; total_size];

        // reading the rest of the data into this new buffer
        self.file.read_exact(&mut payload)?;

        // now that the payload contains the [KEY][VALUE] together into one thing. we will exact
        // value from it which is essentially the start from klen to the end
        let value_bytes = &payload[klen..];

        // finally converting the raw bytes back into the string
        // we use unwrap here to handle if hte bytes aint valid
        // since value_bytes is just a slice or temporary view of the array, we use
        // String::from_utf8 to own the data and turn it into array with to_Vec() and then to
        // string
        let value = String::from_utf8(value_bytes.to_vec()).map_err(|e| io::Error::new(io::ErrorKind::InvalidData,e))?;

        Ok(Some(value))


    } 
    // we keep this private because only our engine should call this, this is like utility function
    // the purpose of load method is to read only the keys and their offset on the go and read the
    // values only when needed
    // basically-
    // start from byte 0
    // read the header (24 bytes) -> extract key_len and val_len
    // read the key -> store (key, current offset) in the hashmap
    // we keep seeking forward by val_len bytes
    // repeat until we hit EOF

    fn load(&mut self) -> io::Result<()>{
        let mut current_pos = 0;

        self.file.seek(SeekFrom::Start(0))?;

        loop {
            // we try to read the header
            let mut header = [0u8;24];

            // basically a graceful handle method such that if this fails in EOF issues, we just
            // break it
            match self.file.read_exact(&mut header) {
                Ok(_) =>{},
                Err(ref e) if e.kind() == io:ErrorKind::UnexpectedEof=>{
                    // end of file error so we're good 
                    break;
                }
                Err(e) => return Err(e), // now this is serious
            }

            let klen_bytes : [u8, 4] = header[16..20].try_into().unwrap();
            let klen = u32::from_be_bytes(klen_bytes) as usize;

            let vlen_bytes : [u8, 4] = header[20..24].try_into().unwrap();
            let vlen = u32::from_be_bytes(vlen_bytes) as usize;

            
            let mut key_buffer = vec![0u8; klen];
            self.file.read_exact(&mut key_buffer)?;


            let key= String::from_utf8(key_buffer).map_err(|e| io::Error::new(io::ErrorKind::InvalidData,e))?;

            // offset is where the entry started 
            self.index.insert(key, current_pos);

            // we advance 'current_pos' by the total size of this entry
            let total_size = 24+klen+vlen;

            // seek the cursor past the value
            self.file.seek(SeekFrom::Current(vlen as i64))?;

            // update our tracker
            current_pos += total_size as u64;

        }

        Ok(())
    }
    // this is supposed to be prviate 
    fn load(&mut self) -> io::Result<()> {
        let mut current_pos = 0;

        self.file.seek(SeekFrom::Start(0))?;

        loop{
            // try to reard the header
            let mut header = [0u8; 24];

            // handle EOF gracefullyo
            match self.file.read_exact(&mut header){
                Ok(_) =>{},
                Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof=>{
                    break;
                }
                Err(e) => return Err(e);
            }
            
            let klen_bytes: [u8; 4] = header[16..20].try_into().unwrap();
            let kln = u32::from_be_bytes(klen_bytes) as usize;

            let vlen_bytes: [u8; 4] = header[16..20].try_into().unwrap();
            let vln = u32::from_be_bytes(vlen_bytes) as usize;

            // read the key for the hashmape
            let mut key_buffer = vec![0u8; klen];
            self.file.read_exact(&mut key_buffer)?;

            let key = String::from_utf8(key_buffer)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            // key starts at 'current_pos'
            self.index.insert(key, current_pos)
            

        }

    }

}

// now we will create test as usual to -
// 1. Index lookup for checking ram to find the address 
// 2. seek by moving the disk head to that address
// 3. read the first 24 bytes to get the size
// 4. read the exact data bytes
// 5. convert those bytes back to strings


#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn test_kv_store(){
        let path = "test_database.db"

        let _ = std::fs::remove_file(path);

        let mut store = RustyKV::open(path).expect("Failed to open the DB")

        store.set("key1".to_string(), "value1".to_string()).expect("Failed to set");

        store.set("key2".to_string(), "value2".to_string()).expect("Failed to set");

        let val1 = store.get("key1".to_String()).expect("Failed to get").unwrap();
        let val2 = store.get("key2".to_String()).expect("Failed to get").unwrap();

        assert_eq!(val1, "value1");
        assert_eq!(val2, "value2");

        let missing = store.get("key3".to_string()).expect("Failed to get");

        assert_eq!(missing, None);

        std::fs::remove_file(path).unwrap();

    }
}

#[test]
fn test_persistence() {
    let path = "persistent_test.db";
    let _ = std::fs::remove_file(path); //clean start
    // write the data 
    {
        let mut store = RustyKV::open(path).expect("Failed to open DB");
        store.set("persistent_key".to_string(), "persistent_value".to_string()).expect("Failed to set");
        // when this block ends, 'store' is dropped (var is destroyed)
        // this simulates closing the program
    }
    {
        let mut store = RustyKV::open(path).expect("Failed to reopen DB");

        // at this point, the hashmap entry is memory
        // but store.load() should have run and refilled it 
        let val = store.get("persistent_key".to_string()).expect("Failed to get").unwrap();
        assert_eq!(val, "persistent_value");

    }

    std::fs::remove_file(path).unwrap();
}

















