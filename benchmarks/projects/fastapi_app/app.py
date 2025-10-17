from fastapi import FastAPI
import uvicorn
app = FastAPI()

@app.get("/")
def root():
    return {"msg": "Hello FastAPI"}

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
    uvicorn.run(app)


