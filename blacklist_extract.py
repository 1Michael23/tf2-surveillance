import sys

def extract_addresses(input_file, output_file):
    addresses = []

    with open(input_file, "r", encoding='iso-8859-1') as file:
        lines = file.readlines()
        for line in lines:
            if '"addr"' in line:
                address = line.split('"')[-2]
                addresses.append(address)

    with open(output_file, "w") as file:
        for address in addresses:
            file.write(f"{address}\n")

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: python script.py <input_file> <output_file>")
        sys.exit(1)
    input_file = sys.argv[1]
    output_file = sys.argv[2]
    extract_addresses(input_file, output_file)

