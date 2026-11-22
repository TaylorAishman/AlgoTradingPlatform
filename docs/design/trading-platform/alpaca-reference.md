# Alpaca behaviour: what the documentation states

Reference for `trading-platform` build tasks. Written 2026-11-22 in answer to the plan's open question PQ-3. It records what Alpaca's own documentation says about each behaviour the code relies on, so that fixtures and adapters are written from stated behaviour, and it narrows the paper-account probes of R-190 (#106) to what the documentation leaves open.

Sources were read on 2026-11-22. Quotes are exact. "Alpaca Learn" is Alpaca's own article site (alpaca.markets/learn), not its API documentation, and is labelled wherever it is used. Community forum posts were not used as a source.

## Summary

| # | Behaviour | Settled by the docs? | Code should | Tickets |
| --- | --- | --- | --- | --- |
| 1 | Delayed SIP history for a paper-only account | No: pages disagree | Try SIP, fall back to IEX (R-290, R-317) | #79, #85, #106 |
| 2 | Response when a feed is not covered | Mostly: 403 and a message | Match HTTP 403 and the message text, not the numeric code | #31, #79 |
| 3 | How far back quote history reaches | No | Treat missing early quotes as missing samples (R-296, R-302) | #85, #106 |
| 4 | Scope of the 200 calls per minute | Number only | One limiter for all data endpoints (R-043) | #66, #106 |
| 5 | Duplicate `client_order_id` | Behaviour yes; body only in Alpaca Learn | Treat 422 with "client_order_id must be unique" as already submitted, then look up | #62, #106 |
| 6 | Calendar endpoint | Yes, except time zone | Read times as America/New_York | #31, #106 |
| 7 | Opening-auction (`opg`) orders | Window and whole shares yes; paper fill price no | Submit between 19:00 and 09:28 ET, whole shares only | #49, #60, #106 |

## 1. Market data for a paper-only account

The pages disagree:

- [Paper trading](https://docs.alpaca.markets/docs/paper-trading): "As an Alpaca Paper Only Account holder, you are only entitled to receive and make use of IEX market data."
- [About Market Data API](https://docs.alpaca.markets/docs/about-market-data-api): "The **Basic** plan serves as the default option for both Paper and Live trading accounts". Its equities table gives Basic "Historical data timeframe | Since 2016" and "Historical data limitation* | latest 15 minutes".
- [Market data FAQ](https://docs.alpaca.markets/docs/market-data-faq): "For historical queries, the `end` parameter must be at least 15 minutes old to query SIP data without a subscription."
- [Historical stock data](https://docs.alpaca.markets/docs/historical-stock-data-1), `iex` row: "This is the only feed that can be used without a subscription."

No page reconciles them. **Open: probe 1 of R-190.** The design already handles both outcomes: SIP is tried first and a refusal moves the job to IEX (R-290, R-317, R-318).

## 2. Refusal response

- [Historical bars](https://docs.alpaca.markets/reference/stockbars) and [historical quotes](https://docs.alpaca.markets/reference/stockquotes-1) both define 403 as "The requested resource is forbidden, for example because the subscription does not cover the requested data." No response body schema is given.
- Both pages give `end` the default "the current time if the user has a real-time access for the feed, otherwise 15 minutes before the current time."
- [Market data FAQ](https://docs.alpaca.markets/docs/market-data-faq): "The market data endpoints return HTTP 403 if any of the following conditions are true: … the authenticated user has insufficient permissions", and "if you get a message like `subscription does not permit querying recent SIP data` in the HTTP response body, make sure you have the proper subscription". Its example body is `{"code":42210000,"message":"subscription does not permit querying recent SIP data"}`.
- Alpaca Learn ([common errors](https://alpaca.markets/learn/how-to-fix-common-trading-api-errors-at-alpaca)) shows the same message with status 403 and code `40010001`.

The two sources give different numeric codes. **Code should** recognise a refusal by HTTP 403 together with a message containing "subscription does not permit", and must not depend on the numeric code. Fixtures for #31 and #79 should use a 403 with that message. **Open:** the exact body for a historical (not latest) SIP request that a paper-only account may not make; probe 2 of R-190 records it.

## 3. Quote history depth

The only statement is the About page's "Historical data timeframe | Since 2016", which does not separate bars from quotes or SIP from IEX. The quotes reference gives no earliest date. **Open: probe 3 of R-190.** R-296 and R-302 already treat a session without quotes as a missing sample, so a shorter quote history degrades to the fallback spread instead of failing (RQ21).

## 4. Rate limit scope

- About Market Data API: "Historical API calls | 200 / min" for Basic. It does not say whether that is per endpoint, per key or per account.
- Reference pages describe only `X-RateLimit-Limit` and `X-RateLimit-Remaining` headers.
- [Alpaca support](https://alpaca.markets/support/usage-limit-api-calls) (December 2022) for the trading API: "the API is throttled, currently 200 requests per minute, per account", with "429- Too Many Requests" when exceeded.

**Code should** keep R-043's single limiter across all data endpoints (the safe reading) and back off on 429. **Open: probe 4 of R-190** (whether data endpoints share one counter, and whether it is separate from the trading API's).

## 5. Duplicate `client_order_id`

- [Create an order](https://docs.alpaca.markets/reference/postorder): `client_order_id` is "A unique identifier for the order. Automatically generated if not sent. (<= 128 characters)". Documented responses: 200, "403 Forbidden — Buying power or shares is not sufficient.", "422 Unprocessable — Input parameters are not recognized." No duplicate case.
- [Alpaca CLI](https://docs.alpaca.markets/docs/alpacas-cli): "Trading API rejects duplicate submissions with the same `client_order_id`, preventing accidental double-orders in retry logic."
- Alpaca Learn (article above): "If you receive an HTTP 422 error like the one below, it likely means a duplicate client_order_id was used for another active order", with `{"code":40010001,"message":"client_order_id must be unique"}`.
- Lookup: [`GET /v2/orders:by_client_order_id?client_order_id=<id>`](https://docs.alpaca.markets/reference/getorderbyclientorderid), "Retrieves a single order specified by the client order ID." Only the 200 response is documented.

**Code should** look the id up before submitting (R-180) and treat a 422 whose message contains "client_order_id must be unique" as already submitted, then fetch it (R-181). The fixture for #62 should use that body. **Open: probe 5 of R-190**, namely whether ids of filled or cancelled orders can be reused (Alpaca Learn says "another active order") and the not-found response of the lookup.

## 6. Calendar

[`GET /v2/calendar`](https://docs.alpaca.markets/reference/legacycalendar): "The calendar API serves the full list of market days from 1970 to 2029. … the response also contains the specific open and close times for the market days, taking into account early closures." Each entry has `date` (YYYY-MM-DD), `open` and `close` (HH:MM, for example "09:30" and "16:00"), `session_open` and `session_close` (HHMM, for example "0400" and "2000") and `settlement_date`. Parameters: `start`, `end`, `date_type` (TRADING or SETTLEMENT).

The v2 page does not state a time zone. The newer [`GET /v3/calendar/{market}`](https://docs.alpaca.markets/reference/calendar-2) returns `market_timezone` (example "America/New_York") and accepts `timezone`. **Code should** read v2 times as America/New_York. **Open: probe 6 of R-190** confirms it against an early-close day.

## 7. Opening-auction orders

- [Orders at Alpaca](https://docs.alpaca.markets/docs/orders-at-alpaca), time in force `opg`: "This order is eligible to execute only in the market opening auction. Any unfilled orders after the open will be cancelled. OPG orders submitted after 9:28am but before 7:00pm ET will be rejected. OPG orders submitted after 7:00pm will be queued and routed to the following day's opening auction."
- The same page's fractional-orders table marks OPG as not supported, and [fractional trading](https://docs.alpaca.markets/docs/fractional-trading) supports fractional orders "by setting `time_in_force` to `day`".
- [Paper trading](https://docs.alpaca.markets/docs/paper-trading): orders "will be matched against the best available current market price (NBBO)", and "When orders are eligible to be filled, they will receive partial fills for a random size 10% of the time." It does not mention auctions.

**Code should** submit `opg` orders only between 19:00 and 09:28 ET, which the 19:05 ET decide job fits, and only in whole shares (the planner's rounding). Partial paper fills must be expected. **Open: probe 7 of R-190**, namely whether a paper `opg` order fills at the auction price or at the opening quote. Gate 7 is unaffected, because the shadow ledger replays the broker's actual fills (R-187), but the fill-price report of R-188 will show the gap.

## What the owner still runs

With paper keys, `pytest -m alpaca tests/execution/test_alpaca_contract.py` (#106) records items 1 to 7 above. Items 1, 3, 4 and the paper fill price of item 7 have no documented answer, so they are the ones that can change code: a difference from this document becomes one ticket, as plan PQ-3 describes.
