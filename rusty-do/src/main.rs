// TODO : 
// [X] Add ability to delete tasks
// [X] Add ability to add tasks
// [X] Add ability to view tasks
// [X] Add ability to save and load tasks from a file (JSON format)
// [X] Add ability to mark tasks as completed
// [X] Add ability to delete all completed tasks
// [X] Add ability to edit task names
// [X] Add ability to set task priorities (low, medium, high)
// [X] Add ability to change task priorities (low, medium, high)
// [X] Add ability to set deadlines for tasks
// [X] Add ability to edit deadlines for tasks
// [ ] Add ability to sort tasks by priority or deadline
// [ ] Add ability to search tasks by name
// [ ] Optimize functions and code structure

use std::io;
use std::fs;
use serde::{Serialize, Deserialize}; // serde is a popular serialization/deserialization library in
// Rust. We use it to convert our Structs to JSON and back.
// Now we need to tell Rust that our Structs are 
// allowed to be turned into JSON. We do this with 
// a "Macro" called derive
use chrono::NaiveDate; // chrono is a popular date and time library in Rust. we will use it for
                       // deadlines.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum Priority{
    Low,
    Medium,
    High,
}

#[derive(Serialize, Deserialize)]
struct TodoItem{
    id: u64,
    name: String, //Notice we use String and not &str. In a Struct, you generally want the Struct to own its data.
    completed:bool,
    priority: Priority,
    deadline : Option<NaiveDate>, // Option type is used to represent a value that can be either
                                  // Some(value) or None
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

    fn edit_task_name(&mut self, id: u64, new_name: String)->bool{
       // for item in &mut self.items{
        //  if item.id==id{
         //     item.name=new_name;
          //    return true;
          //}
      //}
      // more faster and optimzed way using iterator
      if let Some(item) = self.items.iter_mut().find(|item| item.id==id){
        if  item.name.to_lowercase()==new_name.to_lowercase() {
            return false; // no change
        }
        item.name=new_name;
        println!("Name updated");
        return true;
      }
        false
    }

    fn edit_task_priority(&mut self, id: u64, new_priority: Priority) -> bool {
       if let Some(item) = self.items.iter_mut().find(|item| item.id==id){ 
            item.priority = new_priority;
            println!("Priority updated");
            return true;
       }
        false
    }

    fn complete_item(&mut self, id: u64) -> bool { 
       
        if let Some(item) = self.items.iter_mut().find(|item| item.id==id){
            item.completed = true;
            println!("Task marked as completed");
            return true;
        }
        false
    }
    fn add_item(&mut self, name: String, priority: Option<String>, deadline : Option<String>) -> bool{
        if self.items.iter().any(|item| item.name.to_lowercase()==name.to_lowercase()){
            println!("Task with the same name already exists");
            return false;
        }
       let id = self.next_id;
       let new_item = TodoItem{
            id,
            name,
            completed : false,
            priority: match priority{
                Some(p)=>{
                    match p.to_lowercase().as_str(){
                        "low" => Priority::Low,
                        "medium" => Priority::Medium,
                        "high" => Priority::High,
                        _ => Priority::Medium, // default
                    }
                },
                None => Priority::Medium, // default
            },
            deadline: match deadline{
                Some(d)=>{
                    match NaiveDate::parse_from_str(&d, "%Y-%m-%d"){ // this works because we
                    // imported chrono in format
                    // YYYY-MM-DD 
                        Ok(date) => Some(date),
                        Err(_) => {
                            println!("Invalid date format. Use YYYY-MM-DD");
                            None
                        }
                    }
                },
                None => None,
            },
        };
        self.next_id+=1;
        self.items.push(new_item);
        println!("Task added successfully");
        self.print();
       return true;
    }
    fn delete_all_completed(&mut self){
        self.items.retain(|item| !item.completed);
        println!("All completed tasks deleted");
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
    fn edit_task_deadline(&mut self, id: u64, new_deadline: Option<String>)-> bool{
        if let Some(item) = self.items.iter_mut().find(|item| item.id==id){
            item.deadline = match new_deadline{
                Some(d)=>{
                    match NaiveDate::parse_from_str(&d, "%Y-%m-%d"){
                        Ok(date) => Some(date),
                        Err(_) => {
                            println!("Invalid date format. Use YYYY-MM-DD");
                            return false;
                        }
                    }
                },
                None => None,
            };
            println!("Deadline updated");
            return true;
        }
        false
    }
    fn print(&self){
        println!("================================================================");
            println!("ID\tStatus\tPriority\tDue Date\tName");
        println!("================================================================");
        for item in &self.items{
            let status = if item.completed {"[X]"} else {"[ ]"};
            let deadline_str = match item.deadline{
                Some(date) => date.to_string(),
                None => "No deadline".to_string(),
            };
            println!("{}\t{}\t{:?}\t\t{:?}\t\t{}",  item.id, status, item.priority, deadline_str,item.name);

        println!("----------------------------------------------------------------");

        }
        println!("================================================================");
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
        println!("1. Task name to add");
        println!("2. View tasks");
        println!("3. Delete the task");
        println!("4. Mark task as completed");
        println!("5. Delete all completed tasks");
        println!("6. Edit task name");
        println!("7. Edit task priority");
        println!("8. Edit task deadline");
        println!("9. Exit");
        let input = get_input();
        match input.as_str() {
            "1"=> {
                println!("Task name!");
                let task_name = get_input();
                println!("Enter priority (low, medium, high) or leave empty for medium");
                let priority_input = get_input();
                let priority = if priority_input.is_empty(){ None } else { Some(priority_input) };
                println!("Enter deadline (YYYY-MM-DD) or leave empty for no deadline");
                let deadline_input = get_input();
                let deadline = if deadline_input.is_empty(){ None } else { Some(deadline_input) };
                if task_name.is_empty(){
                    println!("Cannot add empty task");
                } else if todolist.add_item(task_name, priority, deadline){
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
                println!("Mark task as completed. Enter task Id");
                let input = get_input();
                match input.parse::<u64>(){
                    Ok(id)=>{
                        if todolist.complete_item(id){
                            todolist.save().expect("Failed to save the data");
                            println!("Task marked as completed");
                        } else{
                            println!("Task not found");
                        }
                    },
                    Err(_)=>{
                        println!("Invalid ID number");
                    }
                }
            },
             "5"=>{
                todolist.delete_all_completed();
                todolist.save().expect("Failed to save the data");
                println!("All completed tasks deleted");
            },  "6"=>{
                println!("Enter the task ID");
                let input = get_input();
                match input.parse::<u64>(){
                    Ok(id)=>{
                        println!("Enter new task name");
                        let new_task_name = get_input();
                        if new_task_name.is_empty(){
                            println!("Task name cannot be empty");
                        } else if todolist.edit_task_name(id, new_task_name){
                            todolist.save().expect("Failed to save the data");
                            println!("Task name updated");
                        } else{
                            println!("Task not found");
                        }
                    },
                    Err(_)=>{
                        println!("Invalid ID number");
                    }
                }
            },
            "7"=>{
                println!("Enter the task ID");
                let input = get_input();
                match input.parse::<u64>(){
                    Ok(id)=>{
                        println!("Enter new priority (low, medium, high)");
                        let new_priority_input = get_input();
                        let new_priority = match new_priority_input.to_lowercase().as_str(){
                            "low" => Priority::Low,
                            "medium" => Priority::Medium,
                            "high" => Priority::High,
                            _ => {
                                println!("Invalid priority");
                                continue;
                            }
                        };
                        if todolist.edit_task_priority(id, new_priority){
                            todolist.save().expect("Failed to save the data");
                            println!("Task priority updated");
                        } else{
                            println!("Task not found");
                        }}
                    Err(_)=>{
                        println!("Invalid ID number");
                    },


                }
            },
             "8"=>{
                println!("Enter the task ID");
                let input = get_input();
                match input.parse::<u64>(){
                    Ok(id)=>{
                        println!("Enter new deadline (YYYY-MM-DD) or leave empty for no deadline");
                        let new_deadline_input = get_input();
                        let new_deadline = if new_deadline_input.is_empty(){ None } else { Some(new_deadline_input) };
                        if todolist.edit_task_deadline(id, new_deadline){
                            todolist.save().expect("Failed to save the data");
                            println!("Task deadline updated");
                        } else{
                            println!("Task not found");
                        }},
                    Err(_)=>{
                        println!("Invalid ID number");
                    },
            }
             },"9"=>{
                println!("Bye!");
                break;
            },
            
           _ => println!("Invalid choice!"),
        }
    }

}

