use std::env;
use rusty_kv::RustyKV;

fn main(){
    // collect command line arguments into a vector, rust gives us std::env::args to use it 
     let args: Vec<String> = env::args().collect();

    let mut store = RustyKV::open("database.db").expect("Failed to open DB");

    if args.len() < 2{
        println!("Usage : ");
        println!("  cargo run set <key> <value>");
        println!("  cargo run get <key>");
        return;
    }

    let command = &args[1];

    match command.as_str(){
        "set" => {
            if args.len() < 4{
                println!("Error : 'set' requires a key and a value");
                return;
            }
            let key = args[2].clone();
            let value = args[3].clone();

            store.set(key, value).expect("Failed to set data");
            println!("Ok")
        },
        "get" =>{
            if args.len() < 3 {
                println!("Error get requires key!!!!");
                return;
            }
            let key = args[2].clone();

            match store.get(key){
                Ok(Some(value)) => println!("{}",value),
                Ok(None) => println!("Key not found"),
                Err(e) => println!("Error is {}", e),
            }
        },
        "rm" =>{
            if args.len() < 3{
                println!("Error : rm requires a key");
                return;
            }
            else{
                let key= args[2].clone();
                store.delete(key).expect("Failed to delete");
                println!("ok");
            }

        }
        _ =>{
            println!("Unknown command : {}",command);
        }
    }

}
