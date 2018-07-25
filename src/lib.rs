#![feature(raw_identifiers, futures_api, pin)]

use std::sync::Arc;
use std::fmt::Debug;
use std::collections::HashMap;
use std::borrow::Cow;
use std::marker::Unpin;

use websocket::message::OwnedMessage;
use websocket::server::InvalidConnection;
use websocket::r#async::Server;

use tokio_core::reactor::Handle;
use tokio::executor::thread_pool::ThreadPool;
use futures01::{Future as Future01, Stream as Stream01, Sink as Sink01};
use futures::{Future, Sink, Stream, stream, FutureExt, StreamExt, future, TryStreamExt, TryFutureExt, SinkExt};
use futures::compat::Executor01CompatExt;
use vdom_rsjs::VNode;
use serde::{Serialize, Deserialize};
use serde_derive::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct FullUpdate<A> {
    tree: Arc<VNode<A>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Action<Tag> {
    pub tag: Tag,
    pub associated: HashMap<String, String>,
    #[serde(skip)]
    _private: (),
}

impl<Tag> Action<Tag> {
    pub fn new(tag: Tag) -> Action<Tag> {
        Action { tag, associated: HashMap::new(), _private: () }
    }

    pub fn associate(mut self, name: impl Into<Cow<'static, str>>, prop: impl Into<Cow<'static, str>>) -> Action<Tag> {
        self.associated.insert(name.into().into_owned(), prop.into().into_owned());
        self
    }
}

pub fn serve<ActionTag, ClientSink, ClientStream, NewClient>(handle: Handle, mut new_client: NewClient) -> impl Future<Output = ()>
where ActionTag: Serialize + for<'a> Deserialize<'a> + Send + Debug,
      ClientSink: Sink<SinkItem = Action<ActionTag>, SinkError = ()> + Unpin + Send + 'static,
      ClientStream: Stream<Item = Arc<VNode<Action<ActionTag>>>> + Unpin + Send + 'static,
      NewClient: FnMut() -> (ClientSink, ClientStream) + Clone + 'static,
{
    let server = Server::bind("127.0.0.1:8080", &handle).unwrap();
    let pool = ThreadPool::new();

    let fut = server.incoming()
        .map_err(|InvalidConnection { error, .. }| error)
        .for_each(move |(upgrade, addr)| {
            println!("Got a connection from {}", addr);

            if !upgrade.protocols().iter().any(|s| s == "vdom-websocket-rsjs") {
                pool.spawn(upgrade.reject().map(|_| ()).map_err(|err| println!("error rejecting {:?}", err)));
                return Ok(());
            }

            let (client_sink, client_stream) = new_client();
            let sender = pool.sender();
            let f = upgrade
                .use_protocol("vdom-websocket-rsjs")
                .accept()
                .map_err(|e| println!("error accepting stream: {:?}", e))
                .and_then(move |(ws, _)| {
                    let (ws_sink, ws_stream) = ws.split();
                    let incoming = ws_stream
                        .take_while(|m| Ok(!m.is_close()))
                        .filter_map(|m| match m {
                            OwnedMessage::Ping(_) => {
                                // TODO: Handle pings, going to need to
                                // change these stream/sink pairs to have a
                                // multiplexer in between them to allow
                                // bypassing the client for sending the PONG
                                // response.
                                None
                            }
                            OwnedMessage::Pong(_) => None,
                            OwnedMessage::Text(msg) => {
                                match serde_json::from_str(&msg) {
                                    Ok(action) => Some(action),
                                    Err(err) => {
                                        println!("error deserializing {:?}", err);
                                        None
                                    }
                                }
                            }
                            OwnedMessage::Binary(_) => {
                                println!("unexpected binary message");
                                None
                            }
                            OwnedMessage::Close(_) => {
                                None
                            }
                        })
                        .map_err(|e| println!("error handling ws_stream: {:?}", e))
                        .forward(client_sink.compat(sender.compat()));
                    let outgoing = client_stream
                        .map(|tree| serde_json::to_string(&FullUpdate { tree }).unwrap())
                        .map(|json| OwnedMessage::Text(json))
                        .chain(stream::once(future::ready(OwnedMessage::Close(None))))
                        .map(Ok::<_, ()>)
                        .compat(sender.compat())
                        .forward(ws_sink.sink_map_err(|e| println!("error on ws_sink: {:?}", e)));
                    incoming.join(outgoing)
                });

            pool.spawn(f.map(|_| ()).map_err(|err| println!("accept error: {:?}", err)));
            Ok(())
        })
        .map_err(|err| println!("Server error: {:?}", err));
    ::futures::compat::Compat::from_01(fut).map(|x| x.unwrap())
}
