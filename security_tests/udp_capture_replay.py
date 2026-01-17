import socket, time

LISTEN_IP = "127.0.0.1"
CAPTURE_PORT = 19007
FORWARD_TO_PORT = 19006

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.bind((LISTEN_IP, CAPTURE_PORT))
print(f"[CAPTURE] Listening on {LISTEN_IP}:{CAPTURE_PORT}, forwarding to {LISTEN_IP}:{FORWARD_TO_PORT}")
print("[CAPTURE] Waiting for a packet >150 bytes (likely real HSIP frame)...")

while True:
    data, addr = sock.recvfrom(65535)
    print(f"[CAPTURE] Got {len(data)} bytes from {addr}")
    if len(data) > 150:
        break
    print("[CAPTURE] Too small, ignoring. Send again...")

fwd = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
print(f"[CAPTURE] Using {len(data)} bytes as replay sample. Forwarding once.")
fwd.sendto(data, (LISTEN_IP, FORWARD_TO_PORT))
time.sleep(1)

print("[REPLAY] Replaying exact same datagram 5 times...")
for i in range(5):
    fwd.sendto(data, (LISTEN_IP, FORWARD_TO_PORT))
    print(f"[REPLAY] sent {i+1}/5")
    time.sleep(0.2)

print("[DONE] Watch session-listen output.")
