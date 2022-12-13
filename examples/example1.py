import pywry
from multiprocessing import Process


def readcontents():
    with open("file.html", "r") as f:
        return f.read()


def task(message):
    pywry.show_html(message)


def main():
    for i in range(1):
        p = Process(target=task, args=(readcontents(),))
        p.start()


if __name__ == "__main__":
    main()
