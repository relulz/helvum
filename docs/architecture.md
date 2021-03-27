# Architecture
If you want to understand the high-level architecture of helvum,
this document is the right place.

It provides a birds-eye view of the general architecture, and also goes into details on some
components like the view.

# Top Level Architecture
Helvum uses an architecture with the components laid out like this:

```
┌──────┐
│ View │
└────┬─┘
 Λ   ┆
 │<───── updates view
 │   ┆
 │   ┆<─────────────── notifies of user input (using callbacks)
 │   ┆
 │   ┆
 │   ┆
 │   V           notifies of remote changes
┌┴───────────┐        (using callbacks)      ┌─────────────────────┐
│            │<╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤                     │
│ Controller │                               │ Pipewire Connection │
│            ├──────────────────────────────>│                     │
└┬───────────┘   Request changes to remote   └─────────────────────┘
 │                                                      Λ
 │                                                      ║
 │<─── updates/reads state            Communicates ───> ║
 │                                                      ║
 V                                                      ║
┌───────┐                                               V
│ State │                                   [ Remote Pipewire Server ]
└───────┘
```

The `Controller` struct is the centerpiece of this architecture.
It registers callbacks with the `PipewireConnection` struct to get notified of any changes
on the remote.

For each change it is notified of, it updates the view to reflect those changes, and additionally memorizes anything it might need later in the state.

Additionally, a user may also make changes using the view.
For each change, the view notifies the controller by invoking callbacks registered on it.
The controller will then request the pipewire connection to make those changes on the remote. \
These changes will then be applied to the view like any other remote changes as explained above.

## Control flow
Most of the time, the program will sit idle in a gtk event processing loop.

For any changes made using the view, gtk will emit an event on some widget, which will result in a closure on that widget being called, and in turn the controller being updated.

On the other hand, we may receive updates from the remote pipewire server at any moment. \
To process these changes, the gtk event loop is set up to trigger a roundtrip on the pipewire
connection on an interval. During this roundtrip, we process all events sent to us by the pipewire server and notify the controller of them.

# View Architecture
TODO