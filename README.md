# Notify user about discussions on repo via slack

- bind this to a slack workspace and channel
- set up an owner variable on flows network, the fuction will search all repos for any new discussioons created in the last N days
- use the trigger_word to get this function to work, if not specified in the flow, it defaults to "discuss"