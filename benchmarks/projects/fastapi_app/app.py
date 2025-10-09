from fastapi import FastAPI
import uvicorn
app = FastAPI()

@app.get("/")
def root():
    return {"msg": "Hello FastAPI"}

if __name__ == "__main__":
    uvicorn.run(app)
