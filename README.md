## Notes

```bash
cargo build
```

```bash
cargo test
```

```bash
cargo run --bin schema
```

## About
- cybernet is an effort to incentivize [soft3](https://cyb.ai/oracle/ask/QmTsBLAHC1Lk7n76GX4P3EvbAfNjBmZxwjknWy41SJZBGg) learning
- it is inspired by [yuma consensus](https://github.com/opentensor/developer-docs/blob/bd6833e34fdf3a0a2120be8ab12959f5142728df/docs/yuma-consensus.md) of [bittensor](https://cyb.ai/oracle/ask/QmUwHh7mKJhVMfnnNuDLeDfkUoknHu9FH9bZiS65MaHL72)
- advanced security due to decoupling of layers
	- bostroms [tendermint consensus](https://cometbft.com/) as consensus layer
	- [cosmos-sdk](https://docs.cosmos.network/) with cosmwasm as sequential computation layer
	- [cyber-sdk](https://github.com/cybercongress/go-cyber) as parallel computation layer
	- cybernet experimental reward layer using [cosmwasm programs](https://cosmwasm.com/)
- subtensor is ported from substrate palets to cosmwasm programs
- bittensor is ported to cosmwasm endpoints: [cybertensor](https://github.com/cybercongress/cybertensor)
- protocol is mostly remained untouched for maximum compatibility
- protocol extension: subnetwork is about learning particle's subgraph
- tecnical preview of webapp for exploring and seting weights: https://spacepussy.ai/cybernet
- TODO daodao integration
- TODO enrich original docs of the project
- TODO [cybverver](https://github.com/cybercongress/cyberver) and art created for easier adoption <img width="1025" alt="image" src="https://github.com/cybercongress/cybernet/assets/410789/198c9ed2-5e08-429c-9dfd-268d65cc5728">
- whats is different in comparison with bittensor
	- [deploy you whole new network](https://github.com/cybercongress/cybertensor) and token: the network is just contract instance
	- manage your network using manual ux weights in [tech preview app](https://spacepussy.ai/cybernet/subnets/0)
	- maximize rewards with the help of [cybergraph](https://cyb.ai/oracle/ask/cybergraph)
	- extend subnets using [cosmwasm](https://cosmwasm.com/) programs
  - deploy you [daodao](https://daodao.zone/) instance for subnet management and more
	- participate in vibrant [ibc](https://cosmos.network/ibc/) ecosystem
	- trade earning on permissionless [warp dex](https://cyb.ai/warp)https://cyb.ai/warp
  - enjoy security and speed of [comet consensus](https://cometbft.com/) (former tendermint)
  - and more
