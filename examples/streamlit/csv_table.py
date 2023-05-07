import streamlit as st

csv_file = st.file_uploader("Upload a CSV file", type="csv")


if __name__ == "__main__":
    if csv_file:
        import pandas as pd

        data = pd.read_csv(csv_file)
        st.write(data)
