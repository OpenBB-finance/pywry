import plotly_wry
import threading

def wait_for_user_input():
    message = input('Type html to be viewed in webview:\n> ')
    # spawn thread in python
    threading.Thread(target=task, args=[message]).start()

def task(message):
    plotly_wry.show(message)

def main():
    wait_for_user_input()

if __name__ == '__main__':
    main()