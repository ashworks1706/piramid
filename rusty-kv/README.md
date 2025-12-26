in this project i will make a append only database 

The Concept: Imagine a diary. You never erase a page. You only write on the next empty line.

To save x = 5, you write Set x 5.

To update x = 10, you write Set x 10 (you don't erase the 5).

To delete x, you write Delete x.

The latest entry in the file is the truth.

we will use binary bits packet for the database
every entry in our database will look like a packet of bytes. 

The Header (Fixed Size - 20 Bytes):

CRC (4 bytes): A checksum to detect file corruption.

Timestamp (8 bytes): When was this saved?

Key Size (4 bytes): How long is the key?

Value Size (4 bytes): How long is the value?

The Payload (Variable Size): 5. Key: The actual bytes of the key. 6. Value: The actual bytes of the value.

The Header solves this ambiguity. It tells the computer: "The next 4 bytes are the key. The 5 bytes after that are the value."

lib.rs -> public face of the database
entry.rs -> definition of the data packet


TODO : Build rust kv engine




Bitcask databases or riak databases work by keeping a log disk and index in ram.

we only append to that file, we never go back and overwrtie old data, which is very fast because the disk head just keeps moving forward.
the index in ram is a hashmap, which is just a long list of ytes as a table of contents in memory to know where things are 
     [ RAM: The Index ]                  [ DISK: The Log File ]
  ---------------------------           ---------------------------
  Key       | Location (Offset)         Byte 0:   [Entry: key="A"]
  "user1"   | 0               --------> Byte 50:  [Entry: key="B"]
  "config"  | 50              --------> Byte 120: [Entry: key="C"]


