# SDU Vacant Positions RSS Feed

Subscribe to <a href="https://mtib.dev/sdu-open.xml">SDU Vacant Positions</a> RSS.

## Usage

To generate your own RSS based on the current open positions run ChromeDriver for [fantoccini](https://github.com/jonhoo/fantoccini), then run `cargo run --bin get_sdurss > feed.xml`. You can select the used Chrome binary by setting the `CHROME_BINARY` environment variable. 

This is the example script periodically running on mtib.dev to update the feed:

```sh
#!/bin/bash

(
    ./chromedriver &
    CD="$!"
    sleep 1
    cd sdu_openrss
    CHROME_BINARY="/usr/bin/chromium-browser" cargo run --bin get_sdurss > ../sdu.xml
    kill $CD
)
```

I also wrote [a blog article](https://blog.mtib.dev/sdu-vacancies-rss.html) about my reasoning and progress.