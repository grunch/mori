# Morí
Death is something uncomfortable, usually our death is not something we want to talk about, much less in our youth, but as Bitcoiners we have to prepare our funds for that day that will inevitably come.

This project proposes a way that a btc owner can leave his bitcoins to his heirs in the event of death in a decentralized way and with the least possible complication.

The main characteristic of this proposal is that the heirs can recover the funds in the event of death but CANNOT access the funds while the btc owner lives.

Not knowing when we are going to die is what adds complexity to the matter.

As a Bitcoiner and custodians of our privacy, we need to remove the trusted third party whenever possible.

Bitcoin script allows us to carry out a method so that the heirs cannot access while the btc owner is alive, but they will after his death.

## How does this work?
We will call the btc owner **Alice** and we will call the heir **Bob**:

1. User generates two descriptors, the first one is for Alice, with this descriptor Alice will be able to generate new addresses and spend those UTXO at any moment she need. The second one is for Bob, with this descriptor he will be able to spend every UTXO from Alice after 25920 blocks being mined, this is 6 months approximately.
2. Alicia gives to Bob his descriptor.
3. Alice can use this wallet like any other wallet, but she has to be sure to spend every UTXO before 6 months, if she doesn't we can assume she's dead and Bob can inherit the money.

This is being done with miniscript and we have two spending conditions.

Condition 1: Alice can spend the funds at any time.
Condition 2: Bob can spend the funds after N blocks have been mined.

miniscript policy:
```
or(
    pk(A),
    and(
        pk(B),
        older(25920)
    )
)

```

This first version is a stateless wallet, this means that we are regenerating it every time we run the command line.

## Install and Run
```
$ git clone https://github.com/grunch/mori
$ cd mori
```
We generate two descriptors, they only have a little difference, one will generate the addresses to receive and the other one for the change addresses.

```
$ cargo run -- descriptor
```
Everything before `--` are arguments we are passing to Rust's cargo tool, what is after to `--` are arguments to our program.

We will see our receiving descriptor and our change descriptor, as we are using a stateless wallet we need to pass as arguments those descriptors as parameters everytime, so let's create two variables in a .env file.
```
DESC="wsh(or_d(pk([f7e6924a/87h/1h/0h]tprv8ZgxMBicQKsPdoksgNYN6yy6JxCNTGnHGEcoKwiEM6z8i4v8kZEUzp3UC5LLypQT2mrRTW4Zo4jPTsPmzjTH8MPBTNsHQvbamfxmQRrfoDk/0/*),and_v(v:pk([aab88436/87h/1h/0h]tprv8ZgxMBicQKsPeoE3PXG3hRGDVnSV7fWgnUZ8yaG9JaSQBGqzGEXUyyxj5Dkp4xxbPUZzedjBSghLsoqfuUYukbit47dbkLT3PY3oRXViJGr/0/*),older(25920))))"
CHANGE="wsh(or_d(pk([f7e6924a/87h/1h/0h]tprv8ZgxMBicQKsPdoksgNYN6yy6JxCNTGnHGEcoKwiEM6z8i4v8kZEUzp3UC5LLypQT2mrRTW4Zo4jPTsPmzjTH8MPBTNsHQvbamfxmQRrfoDk/1/*),and_v(v:pk([aab88436/87h/1h/0h]tprv8ZgxMBicQKsPeoE3PXG3hRGDVnSV7fWgnUZ8yaG9JaSQBGqzGEXUyyxj5Dkp4xxbPUZzedjBSghLsoqfuUYukbit47dbkLT3PY3oRXViJGr/1/*),older(25920))))"
```
Now we bring those vars to our shell running `source .env`

To receiving you need to generate a new address, for this we need to pass the descriptor and a index, it starts with 0.
```
cargo run -- receive --index 0 --desc $DESC
```
Send some btc to that address, if you don't have tBTC you can get some from this [faucet](https://bitcoinfaucet.uo1.net/).

To see your balance just run
```
cargo run -- balance --desc $DESC --change $CHANGE
```
Now you want to send some coins to a other user address, for this you need to `build` a transaction.
```
cargo run -- build --desc $DESC --change $CHANGE --amount <amount in satoshsi> --destination <tBTC address>
``eat will generate a [PSBT](https://github.com/bitcoin/bitcoin/blob/master/doc/psbt.md) that we need to sign and broadcast with the `send` command.
```
cargo run -- send --desc $DESC --psbt <psbt transaction>
```
That should show you a transaction Id, that means that it works 😀