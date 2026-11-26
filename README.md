# AlgoTradingPlatform
Experimenting with algorithmic trading, please see README for status and P&amp;L YTD

## API keys

Bring your own Alpaca paper trading keys. Copy the template to a hidden file
that git ignores, fill it in, and point the binary at it:

```sh
cp config/secrets.example.toml config/.secrets.toml
chmod 600 config/.secrets.toml        # the binary refuses a readable file
$EDITOR config/.secrets.toml          # set key_id and secret_key
export PT_SECRETS_FILE="$PWD/config/.secrets.toml"
```

Never commit `config/.secrets.toml`. Exporting `APCA_API_KEY_ID` and
`APCA_API_SECRET_KEY` works too and takes precedence over the file.
