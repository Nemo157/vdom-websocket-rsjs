import diff from 'virtual-dom/diff'
import patch from 'virtual-dom/patch'
import createElement from 'virtual-dom/create-element'
import VPatch from 'virtual-dom/vnode/vpatch'

import * as bridge from 'vdom-rsjs'

function omap(o, f) {
  let n = {}
  for (let [k, v] of Object.entries(o)) {
    n[k] = f(v)
  }
  return n
}

function start() {
  let tree, rootNode
  let socket = new WebSocket("ws://localhost:8080", ["vdom-websocket-rsjs"])

  let onaction = (event, action) => {
    socket.send(JSON.stringify({
      tag: action.tag,
      data: action.data,
      associated: omap(action.associated, v => event.target[v]),
    }))
  }

  socket.onclose = () => {
    document.body.innerHTML = "socket closed"
    setTimeout(() => (document.body.innerHTML = "", start()), 2000)
  }

  socket.onerror = err => {
    document.body.innerHTML = [
      "socket error: ",
      err.toString(),
      JSON.stringify(err),
    ].join("<br>")
  }

  socket.onmessage = msg => {
    try {
      msg = JSON.parse(msg.data)
      if (msg.tree) {
        if (rootNode) {
          let newTree = bridge.node(msg.tree, onaction)
          let patches = diff(tree, newTree)
          tree = newTree
          rootNode = patch(rootNode, patches)
        } else {
          tree = bridge.node(msg.tree, onaction)
          rootNode = createElement(tree)
          document.body.appendChild(rootNode)
        }
      } else {
        document.body.innerHTML = [
          "message without property",
          "msg: " + JSON.stringify(msg),
        ].join("<br>")
      }
    } catch (err) {
      document.body.innerHTML = [
        "onmessage error: ",
        err.toString(),
        JSON.stringify(err),
        "msg: " + msg.data,
      ].join("<br>")
    }
  }
}

start();
