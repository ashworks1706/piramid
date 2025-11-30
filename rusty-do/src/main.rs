// TODO : 
// [X] Add ability to delete tasks
// [X] Add ability to add tasks
// [X] Add ability to view tasks
// [X] Add ability to save and load tasks from a file (JSON format)
// [ ] Add ability to mark tasks as completed
// [ ] Add ability to edit task names
// [ ] Add ability to delete all completed tasks
// [ ] Add ability to prioritize tasks
// [ ] Add ability to set deadlines for tasks
// [ ] Add ability to sort tasks by priority or deadline
// [ ] Add ability to search tasks by name

use std::io;
use std::fs;
use serde::{Serialize, Deserialize}; // serde is a popular serialization/deserialization library in
// Rust. We use it to convert our Structs to JSON and back.
// Now we need to tell Rust that our Structs are 
// allowed to be turned into JSON. We do this with 
// a "Macro" called derive
#[derive(Serialize, Deserialize)]
struct TodoItem{
    id: u64,
    name: String, //Notice we use String and not &str. In a Struct, you generally want the Struct to own its data.
    completed:bool,
}

#[derive(Serialize, Deserialize)]
struct TodoList{
    items: Vec<TodoItem>,
    next_id : u64,
}
// we need a way to create a list and add items to it. We use an impl (implementation) block to 
// define functions associated with our struct
impl TodoList{
    fn new() -> TodoList{
    // TodoList { items: Vec::new(), next_id : 1}
    // Let's try to load from file first
    match fs::read_to_string("db.json"){
        Ok(content)=>{
            match serde_json::from_str(&content){
                Ok(list)=>{
                    list
                }
                Err(_)=>{
                    // if deserialization fails, return empty list
                    TodoList { items: Vec::new(), next_id : 1}
                }
            }
        },
        Err(_)=>{
            // if reading file fails, return empty list
            TodoList { items: Vec::new(), next_id : 1}
        }
    }
    }
    fn add_item(&mut self, name: String) -> bool{
        if self.items.iter().any(|item| item.name.to_lowercase()==name.to_lowercase()){
            return false;
        }
       let id = self.next_id;
       let new_item = TodoItem{
            id,
            name,
            completed : false,
        };
        self.next_id+=1;
        self.items.push(new_item);
       return true;
    }
    fn delete_item(&mut self, id: u64) -> bool{
        // first find if the item exists in the list or not?? WRONG!! rust does not work like that
        // rust prefers -> keep all the todos except the one with ID this!
        let indexes = self.items.iter().position(|item| item.id==id);
        match indexes{
            Some(index)=>{
                self.items.remove(index); // Vec::remove(index) shifts 
                // all elements after the deleted one to the left. 
                // It preserves order but can be slow if the list is 
                // massive (millions of items). For a todo list, it is perfect.
                true
            },
            None => {
                false
            }
        }
    }
    fn print(&self){
        println!("==============================");
        for item in &self.items{
            let status = if item.completed {"[X]"} else {"[ ]"};
            println!("{} {} ---- {}",  item.id, status, item.name);
        }
        println!("==============================");
    }
    
fn save(&self) -> Result<(),std::io::Error>{
    // convert the struct to JSON
    let content = serde_json::to_string_pretty(&self)?; // ? means if the statement fails, return
                                                        // error immediately
    fs::write("db.json",content)?;
    Ok(())
}


}
// Now, let's create a helper function to get input. 
// Why? because reading input in Rust is a three-step process (create buffer -> read line -> handle errors).
fn get_input() -> String{
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer).expect("Failed to read line");
    buffer.trim().to_string() // trim here is for getting rid of \n
}
fn main(){
    let mut todolist = TodoList::new();
    loop{
        println!("Hello!");
        println!("1. Task name to add");
        println!("2. View tasks");
        println!("3. Delete the task");
        println!("4. Exit");
        let input = get_input();
        match input.as_str() {
            "1"=> {
                println!("Task name!");
                let task_name = get_input();
                if task_name.is_empty(){
                    println!("Cannot add empty task");
                } else if todolist.add_item(task_name) {
                    println!("Added task!");
                    todolist.save().expect("Failed to save the data");
                } else{
                    println!("Task already exists!")
                }
            },
            "2" =>{
                println!("Viewing tasks!");
                todolist.print();
            },
            "3"=>{
                println!("Enter task Id to delete");
                let input = get_input();
                let id = input.parse::<u64>().expect("Invalid ID number");
                let res = if todolist.delete_item(id) {
                    todolist.save().expect("Failed to save the data");
                    "Task deleted" 
                }else {
                    "task not found"
                };
                println!("Task deletion {}", res)
            },
            "4"=>{
                println!("Bye!");
                break;
            },
            _ => println!("Invalid choice!"),
        }
    }

}

