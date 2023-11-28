import socket
import threading

HOST = "127.0.0.1"
PORT = 6378

def main():
    client = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    client.connect((HOST, PORT))

    def recv():
        while True:
             data = client.recv(512).decode()
             if not data: sys.exit(0)
             print(data)

    # Start a thread to read data
    threading.Thread(target=recv).start()

    username = input("Insert username: ")
    username.strip()
    username = "${}\r\n".format(username)
    client.sendall(username.encode())

    while True:
        try:
            message = input("> ")
            message.strip()
            message = "#{}\r\n".format(message)
            client.sendall(message.encode())
        except KeyboardInterrupt:
            print('\nBye.')
            break

    # Close the connection
    client.close()

main()
