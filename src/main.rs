use std::collections::{HashMap, HashSet};
use std::time::Instant;
use std::sync::{Arc, RwLock};
// use tokio::sync::{Mutex};
use std::sync::Mutex;
use tokio::sync::broadcast::{self,Sender};
use tokio::net::{TcpListener,TcpStream};
use futures::{SinkExt, StreamExt};
use tokio_util::codec::{FramedRead,FramedWrite,LinesCodec};
use chat_rust_app::random_name;

// const HELP_MSG: &str = include_str!("help.txt");
//this is to include the contents of an external
const HELP_MSG: &str  = "hello this is help";
// #[tokio::main]
// //This is to make the main function asynchronous
// async fn main(){
//     //first of all make a listener or just to start a server
//     let server = TcpListener::bind("127.0.0.1:8080").await.unwrap();
//     // let (mut tcp,_) = server.accept().await.unwrap();
//     //This is the stream that will accept the incoming connections in the server from the client
//     // let mut buffer = [0u8;32];
//     //Buffer that will store the incoming messages from the client to the server.
//     //the loop ensures that the server can connect again and again and do the read write operation continuously
//     loop{
//         let (mut tcp,_) = server.accept().await.unwrap();
//         //When I take this statement inside the loop then the program code will not exit even if the connection one is closed by the client.

//         // let mut buffer = [0u8;32];
//         //The problem in making a buffer and read the input inside it is that we are not sure about the length of the input from the client.

//         //STREAM AND SINK
//         let (reader,writer) = tcp.split();
//         //We can split the Tcpstream into a reader and a writer to be able to write and read to the stream whenever wanted

//         let mut stream = FramedRead::new(reader,LinesCodec::new());
//         let mut sink = FramedWrite::new(writer,LinesCodec::new());
//         //This while loop waits for the next message from the client
//         //stream is basically an iterator
//         while let Some(Ok(mut msg)) = stream.next().await{
//             //ADDING SOME COMMANDS TO THE CHAT APP
//             if msg.starts_with("/help"){
//                 sink.send(HELP_MSG).await.unwrap();
//             }
//             else if(msg.starts_with("/quit")){
//                 break;
//                 //Breaking out of the loop will break the client connection and the server continues the loop
//                 // exit(0);
//                 //Exiting will quit the connection for both client and the server
//                 //to break the connection
//             }
//             sink.send(msg).await.unwrap();
//             //to send the message to the client
//         }

//         // println!("Connected");
//         // let n = tcp.read(&mut buffer).await.unwrap();
//         //This is the read function from the module tokio that will return an asynchronous function
//         // if n==0{
//         //     break;
//         // }
//         // let _ = tcp.write(&buffer[..n]).await.unwrap();
//         //this is to write back to the client which was received by the client
//     }
// }

// #[tokio::main]
// //Handling multiple connections at once concurrently will need separate tasks for each of the connection with the client
// async fn main(){
//     let server = TcpListener::bind("127.0.0.1:8080").await.unwrap();
//     loop{
//         let (mut tcp,_) = server.accept().await.unwrap();
//         tokio::spawn(handle_clients(tcp));
//         //As we spawn different tasks for each of the client and thus we can handle multiple clients at once.
//     }
// }
// async fn handle_clients(mut tcp:TcpStream){
//     let (reader,writer) = tcp.split();
//     let mut stream = FramedRead::new(reader,LinesCodec::new());
//     let mut sink = FramedWrite::new(writer,LinesCodec::new());
//     while let Some(Ok(msg)) = stream.next().await {
//         if(msg.starts_with("/help")){
//             sink.send(HELP_MSG).await;
//             continue;
//         }
//         if(msg.starts_with("/quit")){
//             break;
//         }
//         sink.send(msg).await;
//     }
//     println!("Connection quit");
// }


//Letting the user chat and create rooms
//This is done by letting the users chat with each other using the broadcaast channel.
//The clients just have to subscribe to the broadcast channel and then just subscribe to that broadcast and the clients will be able to exchange the messages.

const MAIN:&str = "main";
struct Room {
    tx: Sender<String>,
}

impl Room {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(32);
        Self {
            tx,
        }
    }
}
#[derive(Clone)]
struct Rooms(Arc<RwLock<HashMap<String, Room>>>);

impl Rooms {
    fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }
    fn join_room(&self, room_name: &str) -> Sender<String> {
        let read_guard = self.0.read().unwrap();
        if let Some(room) = read_guard.get(room_name) {
            return room.tx.clone();
        }
        drop(read_guard);
        let mut write_guard = self.0.write().unwrap();
        let room = write_guard.entry(room_name.to_owned()).or_insert(Room::new());
        room.tx.clone()
    }

    fn list(&self) -> Vec<(String,usize)> {
        let mut list: Vec<_> = self.0.read().unwrap().iter().map(|(name,room)| (name.to_owned(), room.tx.receiver_count()))
        .collect();
    list.sort_by(|a,b| {
        use std::cmp::Ordering::*;
        match b.1.cmp(&a.1){
            Equal => a.0.cmp(&b.0),
            ordering => ordering,
        }
    });
    list
    }
}

#[derive(Clone)]
struct Names(Arc<Mutex<HashSet<String>>>);

impl Names {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(HashSet::new())))
    }
    fn insert(&self, name: String) -> bool {
        //the hashmpa is locked and unlocked and then the elements are inserted and removbed inside it 
        self.0.lock().unwrap().insert(name)
    }
    fn remove(&self, name: &str) -> bool {
        self.0.lock().unwrap().remove(name)
    }
    fn get_unique(&self) -> String {
        let mut name = random_name();
        let mut guard = self.0.lock().unwrap();
        while !guard.insert(name.clone()) {
            name = random_name();
        }
        name
    }
}

#[tokio::main]
async fn main(){
    let server = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    let (tx,_) = broadcast::channel::<String>(30); 
    //the type specifies the type of messages that will be sent in this broadcast channel.
    let mut names = Names::new();
    let mut rooms = Rooms::new();
    loop{
        let (mut tcp,_) = server.accept().await.unwrap();
        tokio::spawn(handle_clients(tcp,tx.clone(),names.clone(),rooms.clone()));
    }
}
async fn handle_clients(mut tcp:TcpStream,tx: Sender<String>,names:Names,rooms:Rooms){
    let (reader,writer) = tcp.split();
    let mut stream = FramedRead::new(reader,LinesCodec::new());
    let mut sink = FramedWrite::new(writer,LinesCodec::new());
    // let mut rx = tx.subscribe();
    //whichever client uses this receiver will be connected to the broadcast channel.
    let mut name = names.get_unique();    //The name for the new client 
    sink.send(format!("You are {}",name)).await.unwrap();
    let mut room_name = MAIN.to_owned();
    let mut room_tx = rooms.join_room(&room_name);
    let mut room_rx = room_tx.subscribe();
    room_tx.send(format!("{name} joined {room_name}"));
    loop {
        tokio::select! {
            msg = stream.next() => {
                let user_msg = match msg{
                    Some(msg) => msg.unwrap(),
                    None => break,
                };
                if(user_msg.starts_with("/help")){
                    sink.send(HELP_MSG).await;
                    continue;
                }

                else if(user_msg.starts_with("/rooms")){
                    let rooms_list = rooms.list();
                    let rooms_list = rooms_list
                    .into_iter()
                    .map(|(name,count)| format!("{name} ({count})"))
                    .collect::<Vec<_>>()
                    .join((", "));
                    sink.send(format!("Rooms -> {rooms_list}")).await;
                }
                else if (user_msg.starts_with("/join")){
                    let new_room = user_msg
                    .split_ascii_whitespace()
                    .nth(1)
                    .unwrap()
                    .to_owned();
                if room_name == new_room{
                    sink.send(format!("You are already inside the room {room_name}")).await;
                    continue;
                }
                room_tx.send(format!("{name} left {room_name}")).unwrap();
                room_tx = rooms.join_room(&new_room);
                room_rx = room_tx.subscribe();
                room_name = new_room;
                sink.send(format!("{name} joined {room_name}"));
                }
                else if(user_msg.starts_with("/name")){

                    let new_name =  user_msg
                    .split_ascii_whitespace()
                    .nth(1)
                    .unwrap()
                    .to_owned();
                let mut changed_name = names.insert(new_name.clone());
                if changed_name{
                    room_tx.send(format!("{name} is now {new_name}")).unwrap();
                    name=new_name;
                }
                else{
                    sink.send(format!("{new_name} is already taken")).await.unwrap();
                }
                }
                else if(user_msg.starts_with("/quit")){
                    break;
                }
                else{
                    room_tx.send(format!("{name}: {user_msg}"));
                }
            },
            //WHEN WE USE THE TOKIO SELECT STATEMENT THEN THE TOKENS WE USE ARE CONVERTED INTO RESULT ENUMS AND THUS WE HAVE TO UNWRAP THEM FIRST BEFORE USING THEM IN THE FUTURES SPAWNED.
            peer_msg = room_rx.recv() => {
                sink.send(peer_msg.unwrap()).await.unwrap();
            },
        //TOKIO SELECT ACTUALLY POLLS MULTIPLE FUTURES AT ONCE
        //This loop is that for getting a message we first have to send a message thus we will add the select message for choosing the event whichever happens first inside the async tasks
        
        //  peer_msg = rx.recv().await.unwrap();
        // sink.send(peer_msg).await.unwrap();

        //when the loop ends the client is disconnected thus he will have left the room
        
        }
    
    }
    room_tx.send(format!("{name} left the {room_name}"));
    names.remove(&name);
}