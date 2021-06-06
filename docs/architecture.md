# Architecture
If you want to understand the high-level architecture of helvum,
this document is the right place.

It provides a birds-eye view of the general architecture, and also goes into details on some
components like the view.

# Top Level Architecture
Helvum uses an architecture with the components laid out like this:

```
┌──────┐
│ GTK  │
│ View │
└────┬─┘
 Λ   ┆
 │<───── updates view                               ┌───────┐
 │   ┆                                              │ State │
 │   ┆<─ notifies of user input                     └───────┘
 │   ┆      (using signals)                             Λ
 │   ┆                                                  │
 │   ┆                                                  │<─── updates/reads state
 │   V           notifies of remote changes             │
┌┴────────────┐         via messages          ┌─────────┴─────────┐
│ Application │<╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌┤     Seperate      │
│   Object    ├╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌>│  Pipewire Thread  │
└─────────────┘   request changes to remote   └───────────────────┘
                        via messages                    Λ
                                                        ║
                                                        ║
                                                        V
                                            [ Remote Pipewire Server ]
```
The program is split between two cooperating threads.
The GTK thread (displayed on the left side) will sit in a GTK event processing loop, while the pipewire thread (displayed on the right side) will sit in a pipewire event processing loop.

The `Application` object inside the GTK thread communicates with the pipewire thread using two channels,
where each message sent by one thread will trigger the loop of the other thread to invoke a callback
with the received message.

For each change on the remote pipewire server, the pipewire thread updates the state and notifies
the `Application` in the GTK thread if changes are needed.
The `Application` then updates the view to reflect those changes.

Additionally, a user may also make changes using the view.
For each change, the view notifies the `Application` by emitting a matching signal.
The `Application` will then ask the pipewire thread to make those changes on the remote. \
These changes will then be applied to the view like any other remote changes as explained above.

# View Architecture
TODO