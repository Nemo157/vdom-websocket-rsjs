#![warn(rust_2018_idioms)]

use std::time::{Instant, Duration};
use std::sync::Arc;

use vdom_rsjs::{VNode, VTag, VProperty};
use vdom_rsjs::render::{Render, Cache, TopCache};
use vdom_websocket_rsjs::Action;
use tokio::timer::Delay;
use tokio_core::reactor::{Core, Handle};
use serde::{Serialize, Deserialize};

use futures::{Sink, Stream, Future, future::{self, Either}};
use futures::sync::mpsc;

#[derive(Serialize, Deserialize, Debug)]
enum ActionTag {
    Increment,
    Decrement,
}

#[derive(Debug)]
struct State {
    count: usize,
    handle: Handle,
    actions: mpsc::Sender<Action<ActionTag>>,
}

impl State {
    fn spawn(handle: Handle) -> (impl Sink<SinkItem = Action<ActionTag>, SinkError = ()>, impl Stream<Item = Arc<VNode<Action<ActionTag>>>, Error = ()>) {
        let (action_tx, action_rx) = mpsc::channel(1);
        let (render_tx, render_rx) = mpsc::channel(1);
        let mut cache = TopCache::new();
        let mut state = Arc::new(State {
            count: 0,
            handle: handle.clone(),
            actions: action_tx.clone(),
        });
        handle.spawn(render_tx.send(cache.render(state.clone()))
            .map_err(|e| println!("state send error: {:?}", e))
            .and_then(|tx| action_rx
                .fold(tx, move |tx, action| {
                    let new_state = state.update(action);
                    if !Arc::ptr_eq(&state, &new_state) {
                        state = new_state;
                        Either::A(tx.send(cache.render(state.clone())).map_err(|e| println!("state send error: {:?}", e)))
                    } else {
                        Either::B(future::ok(tx))
                    }
                })
                .map(|_| ())));
        (action_tx.sink_map_err(|e| println!("error sinking action: {:?}", e)), render_rx)
    }

    fn update(&self, action: Action<ActionTag>) -> Arc<State> {
        let count = match action.tag {
            ActionTag::Increment => self.count + 1,
            ActionTag::Decrement => self.count - 1,
        };

        if count % 2 == 0 {
            let reaction = Action::new(match action.tag {
                ActionTag::Increment => ActionTag::Decrement,
                ActionTag::Decrement => ActionTag::Increment,
            });
            let delay = Instant::now() + Duration::from_millis(200);
            let actions = self.actions.clone();
            self.handle.spawn(
                Delay::new(delay)
                    .map_err(|e| println!("error waiting to send reaction: {:?}", e))
                    .and_then(|()| actions.send(reaction)
                        .map(|_| ())
                        .map_err(|e| println!("error sending reaction: {:?}", e))));
        }

        Arc::new(State {
            count,
            handle: self.handle.clone(),
            actions: self.actions.clone(),
        })
    }
}

impl Render<Action<ActionTag>> for State {
    fn render(&self, _cache: &mut dyn Cache<Action<ActionTag>>) -> VNode<Action<ActionTag>> {
        VTag::new("div")
            .child(self.count.to_string())
            .child(VTag::new("br"))
            .child(VTag::new("button")
                .prop("onclick", VProperty::Action(Action::new(ActionTag::Increment)))
                .child("increment"))
            .child(VTag::new("button")
                .prop("onclick", VProperty::Action(Action::new(ActionTag::Decrement)))
                .child("decrement"))
            .into()
    }
}

fn new_client(handle: Handle) -> (impl Sink<SinkItem = Action<ActionTag>, SinkError = ()>, impl Stream<Item = Arc<VNode<Action<ActionTag>>>, Error = ()>) {
    let (actions, renders) = State::spawn(handle);

    let actions = actions.with(|e| -> Result<Action<ActionTag>, ()> { println!("action: {:?}", e); Ok(e) });
    let renders = renders.map(|r| { println!("render: {:?}", r); r });

    (actions.sink_map_err(|e| println!("error sinking action: {:?}", e)), renders)
}

fn main() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let server = vdom_websocket_rsjs::serve(
        handle.clone(),
        move || new_client(handle.clone()));

    core.run(server).unwrap();
}
