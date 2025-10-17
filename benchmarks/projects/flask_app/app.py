from flask import Flask
app = Flask(__name__)

@app.route("/")
def hello():
    return "Hello from Flask!"
    
if __name__ == "__main__":
    import threading, os
    # Exit the whole process after 1 second (adjust seconds if needed)
    def exit_after_delay(delay_seconds: float = 1.0):
        def _exit():
            print(f"Exiting after {delay_seconds} second(s)")
            os._exit(0)
        t = threading.Timer(delay_seconds, _exit)
        t.daemon = True
        t.start()

    exit_after_delay(1.0)
    app.run(host="127.0.0.1", port=5000)
