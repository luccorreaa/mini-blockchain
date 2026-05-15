pub async fn help() -> &'static str {
    "GET  /chain          — returns the full blockchain as JSON\n\
     GET  /validate       — validates chain integrity\n\
     GET  /block/:index   — returns a block by index\n\
     POST /transaction    — submits a pre-signed transaction\n\
     POST /mine           — mines pending transactions into a block\n\
     GET  /help           — shows this message\n"
}
