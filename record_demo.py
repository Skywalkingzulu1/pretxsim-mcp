import cv2
import mss
import numpy as np
import time
import sys

# --- Config ---
OUTPUT = "C:\\Users\\molel\\pretxsim-mcp\\demo_raw.avi"
FPS = 15
DURATION_SEC = 180  # max 3 minutes
CODEC = cv2.VideoWriter_fourcc(*"MJPG")

with mss.mss() as sct:
    monitor = sct.monitors[1]  # primary monitor
    width = monitor["width"]
    height = monitor["height"]

    out = cv2.VideoWriter(OUTPUT, CODEC, FPS, (width, height))
    if not out.isOpened():
        print("ERROR: Could not open VideoWriter", file=sys.stderr)
        sys.exit(1)

    print(f"Recording {width}x{height} @ {FPS}fps to {OUTPUT}")
    print("Press 'q' to stop early. Max duration: 3 min.")
    print("Starting in 3 seconds...")
    time.sleep(3)

    start = time.time()
    frame_count = 0
    try:
        while True:
            elapsed = time.time() - start
            if elapsed >= DURATION_SEC:
                print("Max duration reached.")
                break

            frame = np.array(sct.grab(monitor))
            frame = cv2.cvtColor(frame, cv2.COLOR_BGRA2BGR)
            out.write(frame)
            frame_count += 1

            if cv2.waitKey(int(1000 / FPS)) & 0xFF == ord("q"):
                print("Stopped by user.")
                break
    finally:
        out.release()
        cv2.destroyAllWindows()

    actual_fps = frame_count / (time.time() - start)
    print(f"Done. {frame_count} frames at ~{actual_fps:.1f} fps.")
    print(f"File: {OUTPUT}")
