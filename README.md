# `vdom-websocket-rsjs`

A pair of Rust crate and npm package that allows rendering to a virtual DOM tree
in Rust, then sending this across a websocket to JavaScript to update the real
DOM.

The Rust crate includes:

 * A tokio based websocket server with a slightly opinionated use of
   [`vdom-rsjs`][] to create an actor like backend per client.

The npm package includes:

 * A basic web page and JavaScript code to connect to any backend using the
   above Rust crate and render it into the DOM.

## Actions

The `vdom-websocket-rsjs` adds slightly more opinion on top of the `vdom-rsjs`
actions, it requires them to consist of a tag that will be sent back and forth
as is, plus some "associated data" that will be read off the event and sent back
with the action. This is definitely not the final form, since you will commonly
want to read data from _some other element_ when an event happens, so this will
be very subject to change.

## Examples

### Counter

This is the canonical virtual DOM example of a counter with increment and
decrement buttons. When the buttons are clicked these push actions over the
websocket to the backend, then the state change is handled server side and the
updated virtual DOM tree is sent back to be rendered to the screen (soon this
will just send the patches back).

The backend will also fight with you to try and keep the counter odd. This is
included as an example of how you can have your backend trigger other actions to
happen then propagate the results into your state update loop.

This is implemented as a separate frontend and backend for build process
simplicity, to build and start the backend run

```sh
$ cargo run --example counter
```

then to build and start the generic `vdom-websocket-rsjs` frontend run

```sh
$ npm install
$ npm start
```

### [`coap-browse`][]

This framework was created as part of building a specific tool, you can take a
look at [`coap-browse`][] to see some more complex usage (including decoupling
the frontend actions from other sorts of events that the backend triggers).

[`vdom-rsjs`]: https://github.com/Nemo157/vdom-rsjs
[`virtual-dom`]: https://npmjs.com/package/virtual-dom
[`coap-browse`]: https://github.com/Nemo157/coap-browse
