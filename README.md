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
- cybernet is an effort to incentivize [[soft3]] learning
- it is inspired by [[yuma consensus]] of [[bittensor]]
- advanced security due to decoupling of layers
	- [[bostrom]] [[tendermint]] as consensus layer
	- [[cosmos-sdk]] with cosmwasm as sequential computation layer
	- [[cyber-sdk]] as parallel computation layer
	- cybernet experimental reward layer using cosmwasm programs
- subtensor is ported from substrate palets to cosmwasm programs
- bittensor is ported to cosmwasm endpoints: [cybertensor](https://github.com/cybercongress/cybertensor)
- protocol is mostly remained untouched for maximum compatibility
- TODO protocol extension: subnetwork is about learning particle's subgraph
- TODO enriched original docs of the project
- TODO cyb app and art created for easier adoption <img width="1025" alt="image" src="https://github.com/cybercongress/cybernet/assets/410789/198c9ed2-5e08-429c-9dfd-268d65cc5728">
- for convenience in app we took metaphor of magic school for gamification
	- network => verse
	- root => root
	- subnetwork => faculty
	- uid => ticket
	- validator => medium
	- delegate => class of medium
	- miner => mage
	- delagator => warrior
- whats is better in comparison with bittensor
	- create you whole new network and token
	- manage your network using manual ux weights in app
	- maximize rewards with the help of [[cybergraph]]
	- program using [[cosmwasm]]
	- participate in vibrant [[ibc]] ecosystem
	- trade against [[H]] and other [[cyber tokens]]
- [[O]] token is for the `cyber` verse
- create verse
	- product
    - explore epochs  
		- explore verses
			- ListDeploidContracts
		- explore verse
			- listSubnetworks
		- explore faculty
			- listSubnetwork
		- deploy verse
			- InstantiateCybernet
				- TODO burn 1 TH
				- TODO pricing for verses
				- max mentors
				- max learners
				- particle with description
		- deploy faculty
			- RegisterNetwork
				- particle:
			- DissolveNetwork
				- netuid: u16
- become mage
	- product
		- mining is hard - good visualization map of steps
			- ```mermaid
				  flowchart LR
				  	subgraph mage [become mage]
				  		ChooseYourVerse[choose verse]-->BurnedRegister[buy ticket]
				          ChooseYourVerse[choose verse]-->Register[mine ticket]
				      end
				      subgraph teach [teach]
				      	BurnedRegister-->ServeAxon[publish axon]
				      	Register-->ServeAxon
				  		ServeAxon-->|optional|ServePrometheus[publish stats]
				          ServeAxon-->|optional|Cyberlink[cyberlink]
				          ServeAxon-->SetWeights[set weights]
				      end
				  	subgraph medium [become medium]
				  	    Cyberlink[cyberlink results]-->BecomeDelegate[attract warriors]
				  		SetWeights-->BecomeDelegate
				          BecomeDelegate-->SetWeightsAsValidator[validate mages]
				  		SetWeightsAsValidator-->RootRegister[prioritize learning]
				  	end
		- amazing personal dashboard of mage
			- Register
				- netuid: u16
				- block_number: u64
				- nonce: u64
				- work: Vec<u8>
				- hotkey: String
				- coldkey: String
			- BurnedRegister
				- netuid: u16
				- hotkey: String
			- ServeAxon
				- netuid: u16
				- version: u32
				- ip: u128
				- port: u16
				- ip_type: u8
				- protocol: u8
				- placeholder1: u8
				- placeholder2: u8
			- ServePrometheus
				- netuid: u16
				- version: u32
				- ip: u128
				- port: u16
				- ip_type: u8
			- RootRegister
				- hotkey: String
			- BecomeDelegate
				- hotkey: String
				- take: u16
		- my verses and faculties with performance
		- set weights for root and faculties
			- SetWeights
				- netuid: u16
				- dests: Vec<u16>
				- weights: Vec<u16>
				- version_key: u64
- become warrior
	- product
		- explore mediums by their apr
		- check consolidated stats on the particular medium
		- check their answers
		- stake on them
			- AddStake
				- hotkey: String
				- amount_staked: u64
			- RemoveStake
				- hotkey: String
				- amount_unstaked: u64
