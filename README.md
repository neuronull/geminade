# geminade

A Rust DCA trading bot for use with the Gemini exchange.

# Usage
 
    $ geminade

# Configuration

cfg.example.toml is an example config.
copy it to cfg.toml.
Edit the file to add your API key/secret and or the Sandbox API key/secret.

Each "strat" has its own set of configurations.

### common configurations

    - sandbox = true/false        # use the sandbox url and API key
    - immediate = true/false      # execute the strat loop immediately
    - symbol = "<symbol>"         # string representation of the trading symbol pair as defined at https://docs.gemini.com/rest-api/
    - trade_day = u32             # day of the week to execute strat loop
    - trade_hr = u32              # hour of the day to execute strat loop
    - trade_min = u32             # min of the hour to execute strat loop
    - trade_sec = u32             # sec of the min to execute strat loop

### strat = "static"

This is a simple periodic weekly buy.

    - usd_per_trade = f32         # amount of USD to use in limit order

### strat = "dips"

This is a simple algorithm to buy the dips.
It only places orders at each 1% of the current ask down to 15% of the current ask.
This guarantees at least one buy for the week.
If during the week, the price does not move to one of the order's prices, the usd value of that order is rolled over into the next order.
Each % has it's own usd tracked so if the price does not fall 15% during week one but does in week twelve, then the 15% order will be made for 12x the order at 1%.

Note this program currently assumes the account is funded.

    - usd_per_window = f32         # amount of USD to split across the 15 orders for the week.


# Logs

Logging is provided via log4rs (https://github.com/estk/log4rs)
Directory containing logfiles is logs/
Log files are separated by module per logcfg.tml.

# Features

- [x] Multithreaded support for multiple strategies in parallel.
- [x] Logging to file
- [x] Cancel open orders when CTL-C detected

# TODO

- [ ] consider a strat which subscribes to websocket events and adapts to market movements autonomously.
