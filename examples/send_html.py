from pywry import PyWry

# We set daemon=False so that the process doesn't close immediately.
handler = PyWry(daemon=False)

if __name__ == "__main__":
    try:
        handler.send_html("<h1 style='color: red;'>Welcome to plotting in PyWry</h1>")
        handler.start()
    except KeyboardInterrupt:
        handler.close()
