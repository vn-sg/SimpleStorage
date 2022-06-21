# SimpleStorage

This is a demo of the fundamental infrastructure of the smart contract of SimpleStorage, which mainly relies on IBC-packets and relayer to relay messages across blockchains.

## Installation

Use the package manager [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) to install dependencies and crates required to run Rust code and unit tests

```bash
curl https://sh.rustup.rs -sSf | sh
```

## Usage

First, we need to have 3 wasmd blockchains up and running in order to test the basic functionality of the smart contract. To do this, simply do:
```bash
./start3chains
```
This will automatically ensure you have wasmd correctly installed and set up 3 testing blockchains running in the background. This script will also initialize IBC relayer and generate appropriate configurations. Keys and relayer paths will also be updated accordingly.
<br><br>
To upload this smart contract to each blockchain, 
```bash
./run upload
```
The above command will compile and optimize the smart contract into minimal file size. Then it is uploaded onto each blockchain and instantiated.
<br><br>
Afterwards, use
```bash
./run link
```
to link the ibc-ports of blockchains to enable interblockchain communication.
<br><br>
### **Start the Relayer**
- Upon completing the above setup steps, you would then be able to start the relayer and relay packets across testing blockchains by the command:
```bash
 rly start mypath0-1 --debug-addr localhost:7597
 rly start mypath0-2 --debug-addr localhost:7598
```
be sure to supplement different ports to avoid conflict.

Now, execute the contract using following command or type your own execution
```bash
# execute
./run e

# query current state of blockchain0
./run q 0 state

# query the tx with id 0 on blockchain0
./run q 0 tx 0
```

## Contributing
Pull requests are welcome. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License
[MIT](https://choosealicense.com/licenses/mit/)