import plotly_wry
from multiprocessing import Process

def readcontents():
    with open('file.html', 'r') as f:
        return f.read()

def wait_for_user_input():
    while True:
        message = input('Type html to be viewed in webview:\n> ')
        p = Process(target=task, args=(message,))
        p.start()

def task(message):
    plotly_wry.show_html(message)

def main():
    for i in range(4):
        p = Process(target=task, args=(readcontents(),))
        p.start()

if __name__ == '__main__':
    main()