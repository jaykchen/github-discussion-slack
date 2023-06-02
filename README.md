# Notify user about discussions on repo via slack

please set a time to invoke this flow with the following format
- time_to_invoke is a string of 3 numbers separated by spaces, representing minute, hour, and day
- \* is the spaceholder for non-specified numbers

bind this to a slack workspace and channel

set up an owner variable on flows network, the fuction will search all repos for any new discussions created in the last N days