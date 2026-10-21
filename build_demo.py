import cv2
import sys
import os

REPO = "C:\\Users\\molel\\pretxsim-mcp"
IMAGE = f"{REPO}\\architecture.jpg"
VIDEO = f"{REPO}\\screen-capture (1)_2.mp4"
OUTPUT = f"{REPO}\\pretxsim_demo_final.mp4"

INTRO_SECONDS = 6.5
OUTPUT_FPS = 30.0

print("Opening screen capture video...")
cap = cv2.VideoCapture(VIDEO)
if not cap.isOpened():
    print(f"ERROR: Could not open {VIDEO}", file=sys.stderr)
    sys.exit(1)

width = int(cap.get(cv2.CAP_PROP_FRAME_WIDTH))
height = int(cap.get(cv2.CAP_PROP_FRAME_HEIGHT))
source_fps = cap.get(cv2.CAP_PROP_FPS)
screen_total_frames = int(cap.get(cv2.CAP_PROP_FRAME_COUNT))
source_duration = screen_total_frames / source_fps
print(f"Source: {width}x{height} @ {source_fps:.3f}fps, {screen_total_frames} frames, {source_duration:.1f}s")

print("Loading architecture slide...")
img = cv2.imread(IMAGE)
if img is None:
    print(f"ERROR: Could not load {IMAGE}", file=sys.stderr)
    sys.exit(1)
img = cv2.resize(img, (width, height), interpolation=cv2.INTER_AREA)
print(f"Slide resized to {width}x{height}")

fourcc = cv2.VideoWriter_fourcc(*"mp4v")
out = cv2.VideoWriter(OUTPUT, fourcc, OUTPUT_FPS, (width, height))
if not out.isOpened():
    print("ERROR: Could not open VideoWriter", file=sys.stderr)
    sys.exit(1)

# Intro: 6.5 seconds at 30fps
intro_frames = int(INTRO_SECONDS * OUTPUT_FPS)
print(f"Writing intro: {intro_frames} frames ({INTRO_SECONDS}s @ {OUTPUT_FPS}fps)")
for i in range(intro_frames):
    out.write(img)

# Screen capture: duplicate frames to preserve original duration at 30fps
# Each source frame needs 30/source_fps output frames to maintain timing
frames_per_source = OUTPUT_FPS / source_fps
print(f"Writing screen capture: {screen_total_frames} source frames, duplicating {frames_per_source:.1f}x each")
print(f"Expected screen duration: {source_duration:.1f}s")

count = 0
src_idx = 0
while True:
    ret, frame = cap.read()
    if not ret:
        break
    # Write this source frame multiple times to preserve timing
    repeats = int(round(frames_per_source))
    for _ in range(repeats):
        out.write(frame)
    count += 1
    if count % 100 == 0:
        print(f"  {count} source frames processed...")

cap.release()
out.release()

total_frames = intro_frames + int(screen_total_frames * frames_per_source)
duration = total_frames / OUTPUT_FPS
size_mb = os.path.getsize(OUTPUT) / (1024 * 1024)
print(f"\nDone! Output: {OUTPUT}")
print(f"Size: {size_mb:.1f} MB")
print(f"Total frames: {total_frames}")
print(f"Duration: {duration:.1f} seconds ({duration/60:.1f} minutes)")
print(f"Intro: {INTRO_SECONDS}s + Screen: {source_duration:.1f}s = {INTRO_SECONDS + source_duration:.1f}s")
