/*
 * This is a set of helpers meant for use with @cosmjs/cli
 * With these you can easily use the bonsai contract without worrying about forming messages and parsing queries.
 *
 * Usage: npx @cosmjs/cli --init https://github.com/bragaz/wasm-test-contract/tree/master/helper.ts
 *
 * Create a client:
 *   const client = await useOptions(heldernetOptions).setup(password);
 *   await client.getAccount()
 *
 * Get the mnemonic:
 *   await useOptions(heldernetOptions).recoverMnemonic(password)
 *
 * If you want to use this code inside an app, you will need several imports from https://github.com/CosmWasm/cosmjs
 */

interface Options {
  readonly httpUrl: string
  readonly networkId: string
  readonly feeToken: string
  readonly gasPrice: string
  readonly bech32prefix: string
}

const defaultOptions: Options = {
  httpUrl: 'https://rpc.musselnet.cosmwasm.com',
  networkId: 'musselnet',
  feeToken: 'umayo',
  gasPrice: '0.025umayo',
  bech32prefix: 'wasm',
}

const defaultFaucetUrl = 'https://faucet.musselnet.cosmwasm.com/credit'

const defaultGasPrice = GasPrice.fromString("0.025umayo");
const defaultGasLimits: GasLimits<CosmWasmFeeTable> = {
  upload: 1_500_000,
  init: 500_000,
  migrate: 200_000,
  exec: 200_000,
  send: 80_000,
  changeAdmin: 80_000,
};

const buildWallet = (mnemonic: string): Promise<Secp256k1HdWallet> => {
  return Secp256k1HdWallet.fromMnemonic(mnemonic, makeCosmoshubPath(0), defaultOptions.bech32prefix);
}

const randomAddress = async (): Promise<string> => {
  const mnemonic = Bip39.encode(Random.getBytes(16)).toString()
  return mnemonicToAddress(mnemonic)
}

const mnemonicToAddress = async (
  mnemonic: string
): Promise<string> => {
  const wallet = await buildWallet(mnemonic);
  const [{ address }] = await wallet.getAccounts()
  return address
}

const getAttibute = (
  logs: readonly logs.Log[],
  key: string
): string | undefined =>
  logs[0].events[0].attributes.find((x) => x.key == key)?.value

const hitFaucet = async (
  faucetUrl: string,
  address: string,
  denom: string
): Promise<void> => {
  const r = await axios.post(defaultFaucetUrl, { denom, address })
  console.log(r.status)
  console.log(r.data)
}

const connect = async (
  mnemonic: string,
  opts: Partial<Options>
): Promise<{
  client: SigningCosmWasmClient
  address: string
}> => {
  const options: Options = { ...defaultOptions, ...opts }
  const wallet = await buildWallet(mnemonic)
  const [{ address }] = await wallet.getAccounts()

  const client = new SigningCosmWasmClient(
    options.httpUrl,
    address,
    wallet,
    defaultGasPrice,
    defaultGasLimits,
  )
  return { client, address }
}

interface Bonsai {
  readonly id: string,
  readonly birth_date: number,
  readonly price: Coin;
}

interface BonsaiList {
  readonly bonsais: Bonsai[];
}

interface Gardener {
  readonly name: string,
  readonly address: string,
  readonly bonsais: Bonsai[];
}

interface AllGardenersResponse {
  readonly gardeners: Gardener[];
}

interface InitMsg {
  readonly price: Coin,
  readonly number: number,
}

interface BonsaiInstance {
  readonly contractAddress: string

  // queries
  getBonsais: () => Promise<BonsaiList>
  getGardener: (address?: string) => Promise<Gardener>
  getGardeners: () => Promise<AllGardenersResponse>

  // actions
  becomeGardener: (name: string) => Promise<string>
  buyBonsai: (b_id: string) => Promise<string>
  sellBonsai: (recipient: string, b_id: string) => Promise<string>
  cutBonsai: (b_id: string) => Promise<string>
}

interface BonsaiContract {
  // upload a code blob and returns a codeId
  upload: () => Promise<number>

  // instantiates a bonsai contract
  // codeId must come from a previous deploy
  // label is the public name of the contract in listing
  // if you set admin, you can run migrations on this contract (likely client.senderAddress)
  instantiate: (codeId: number, initMsg: Record<string, InitMsg>, label: string, admin?: string) => Promise<BonsaiInstance>

  use: (contractAddress: string) => BonsaiInstance
}

const bonsaiCW = (client: SigningCosmWasmClient, metaSource: string, builderSource: string, contractSource: string) : BonsaiContract => {
  const use = (contractAddress: string): BonsaiInstance => {
    const getBonsais = async (): Promise<BonsaiList> => {
      return await client.queryContractSmart(contractAddress, {get_bonsais: {}});
    }

    const getGardener = async (address?: string): Promise<Gardener> => {
      const sender = address || client.senderAddress;
      return await client.queryContractSmart(contractAddress, {get_gardener: {sender}});
    }

    const getGardeners = async () : Promise<AllGardenersResponse> => {
      return await client.queryContractSmart(contractAddress, {get_gardeners: {}});
    }

    const becomeGardener = async (name: string): Promise<string> => {
      const result = await client.execute(contractAddress, {become_gardener:{name}});
      return result.transactionHash;
    }

    const buyBonsai = async (b_id: string) : Promise<string> => {
      const result = await client.execute(contractAddress, {buy_bonsai:{b_id}});
      return  result.transactionHash;
    }

    const sellBonsai = async(recipient: string, b_id: string): Promise<string> => {
      const result = await  client.execute(contractAddress, {sell_bonsai:{b_id, recipient}});
      return result.transactionHash;
    }

    const cutBonsai = async(b_id: string): Promise<string> => {
      const result = await  client.execute(contractAddress, {cut_bonsai:{b_id}});
      return result.transactionHash;
    }

    return {
      contractAddress,
      getBonsais,
      getGardener,
      getGardeners,
      becomeGardener,
      buyBonsai,
      sellBonsai,
      cutBonsai,
    };
  }

  const downloadWasm = async (url: string): Promise<Uint8Array> => {
    const r = await axios.get(url, { responseType: 'arraybuffer' })
    if (r.status !== 200) {
      throw new Error(`Download error: ${r.status}`)
    }
    return r.data
  }

  const upload = async (): Promise<number> => {
    const meta = {
      source: metaSource,
      builder: builderSource
    };
    const wasm = await downloadWasm(contractSource);
    const result = await client.upload(wasm, meta);
    return result.codeId;
  }

  const instantiate = async (codeId: number, initMsg: Record<string, InitMsg>, label: string, admin?: string): Promise<BonsaiInstance> => {
    const result = await client.instantiate(codeId, initMsg, label, { memo: `Init ${label}`, admin});
    return use(result.contractAddress);
  }

  return { upload, instantiate, use };
}

// Example:
// const mnemonic = "use favorite source endless faculty sauce clean core below squirrel profit creek either sign chef giggle ahead stool secret mouse prepare oven more item"
// const result = connect(mnemonic, defaultOptions)
// const metaSourcePath = "https://github.com/bragaz/wasm-test-contract/tree/v0.2.1"
// const optimizerPath = "cosmwasm/rust-optimizer:0.10.7"
// const sourceUrl = "https://github.com/bragaz/wasm-test-contract/releases/download/v0.2.1/my_first_contract.wasm"
// const resolvedResult = await result
// hitFaucet(defaultFaucetUrl, resolvedResult.address, defaultOptions.feeToken)
// const factory = bonsaiCW(resolvedResult.client, metaSourcePath, optimizerPath, sourceUrl)
// const codeId = await factory.upload();
// const contract = await factory.instantiate(codeId, {price: {denom: "ucosm", amount: "5"}, number: 5}, "Bonsai")
// contract.contractAddress -> 'coral1267wq2zk22kt5juypdczw3k4wxhc4z47mug9fd'
//
// OR
//
// const contract = factory.use('coral1267wq2zk22kt5juypdczw3k4wxhc4z47mug9fd')
//
