#[tokio::main]
async fn main() {
    env_logger::Builder::new().filter_level(log::LevelFilter::Debug).init();
    log::info!("Starting server: function: main()");

    let listen_address: &str = "secret";
    let tcp_listener: tokio::net::TcpListener = tokio::net::TcpListener::bind(listen_address).await.unwrap();

    let shared_number: std::sync::Arc<std::sync::Mutex<u64>> = std::sync::Arc::new(std::sync::Mutex::new(0));
    let (tx_sender, _rx_receiver) = tokio::sync::broadcast::channel::<u64>(10);

    {
        let shared_number_clone: std::sync::Arc<std::sync::Mutex<u64>> = std::sync::Arc::clone(&shared_number);
        let tx_sender_clone: tokio::sync::broadcast::Sender<u64> = tx_sender.clone();

        tokio::spawn(async move {
            let mut time_interval: tokio::time::Interval = tokio::time::interval(std::time::Duration::from_secs(5));

            loop {
                time_interval.tick().await;
                let mut number: std::sync::MutexGuard<'_, u64> = shared_number_clone.lock().unwrap();
                *number += 1;

                match tx_sender_clone.send(number.clone()) {
                    Ok(_ok) => {
                        log::info!("Broadcast: tx_sender_clone.send(number.clone()) | Function: main() | success");
                    },
                    Err(error) => {
                        log::error!("FAILURE | Broadcast: tx_sender_clone.send(number.clone()) | Function: main() | ERROR: {}", &error);
                    },
                };
            }
        });
    }

    while let Ok((tcp_stream, _)) = tcp_listener.accept().await {
        let shared_number_clone: std::sync::Arc<std::sync::Mutex<u64>> = std::sync::Arc::clone(&shared_number);
        let rx_receiver: tokio::sync::broadcast::Receiver<u64> = tx_sender.subscribe();
        log::info!("spawn handle_lobby | rx_receiver: {:?} | shared_number_clone: {:?} | tcp_stream: {:?}", &rx_receiver, &shared_number_clone, &tcp_stream);
        tokio::spawn(handle_lobby(tcp_stream, shared_number_clone, rx_receiver));
    }
}

async fn handle_lobby(stream: tokio::net::TcpStream, number: std::sync::Arc<std::sync::Mutex<u64>>, mut rx: tokio::sync::broadcast::Receiver<u64>) {
    log::info!("function: handle_lobby() | New web socket established");
    let web_socket_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream> = tokio_tungstenite::accept_async(stream).await.unwrap();
    let (mut stream_write, mut stream_read) = futures::StreamExt::split(web_socket_stream);

    {
        let initial_message: String = {
            let unwrapped_number: std::sync::MutexGuard<'_, u64> = number.lock().unwrap();
            format!("Number: {}", *unwrapped_number)
        };

        match futures::SinkExt::send(&mut stream_write, tokio_tungstenite::tungstenite::Message::Text(initial_message.into())).await {
            Ok(_ok) => {
                log::warn!("Initial message SUCCESSFUL | stream_write | initial_message sent | Function: handle_lobby");
            },
            Err(error) => {
                log::error!("FAILURE | stream_write | initial_message not sent | Function: handle_lobby | calling return | ERROR: {}", &error);
                return;
            },
        }
    }

    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(recieved_number) => {
                    let reoccuring_message: String = format!("Number: {}", &recieved_number);

                    match futures::SinkExt::send(&mut stream_write, tokio_tungstenite::tungstenite::Message::Text(reoccuring_message.into())).await {
                        Ok(_ok) => {
                            log::warn!("Re-occuring message SUCCESSFUL | stream_write | reoccuring_message sent | Function: handle_lobby : tokio::spawn(async move)");
                        },
                        Err(error) => {
                            log::error!("FAILURE | stream_write | reoccuring_message not sent | Function: handle_lobby : tokio::spawn(async move) | calling break | ERROR: {}", &error);
                            break;
                        },
                    }
                },
                Err(error) => {
                    log::error!("FAILURE | rx.recv() | Recieved: RecvError | Function: handle_lobby : tokio::spawn(async move) | calling break | ERROR: {}", &error);
                    break;
                }
            }
        }
    });

    while let Some(Ok(recieved_message)) = futures::StreamExt::next(&mut stream_read).await {
        log::warn!("Recieved message: {}", &recieved_message)
    }
}