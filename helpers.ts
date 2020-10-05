/*
 * This is a set of helpers meant for use with @cosmjs/cli
 * With these you can easily use the bonsai contract without worrying about forming messages and parsing queries.
 *
 * Usage: npx @cosmjs/cli --init https://github.com/CosmWasm/cosmwasm-plus/blob/master/contracts/cw1-subkeys/helpers.ts
 *
 * Create a client:
 *   const client = await useOptions(coralnetOptions).setup(password);
 *   await client.getAccount()
 *
 * Get the mnemonic:
 *   await useOptions(coralnetOptions).recoverMnemonic(password)
 *
 * If you want to use this code inside an app, you will need several imports from https://github.com/CosmWasm/cosmjs
 */

const path = require("path");

interface Options {
  readonly httpUrl: string
  readonly networkId: string
  readonly feeToken: string
  readonly gasPrice: number
  readonly bech32prefix: string
  readonly hdPath: readonly Slip10RawIndex[]
  readonly faucetToken: string
  readonly faucetUrl?: string
  readonly defaultKeyFile: string
}

const coralnetOptions: Options = {
  httpUrl: 'https://lcd.coralnet.cosmwasm.com',
  networkId: 'cosmwasm-coral',
  feeToken: 'ushell',
  gasPrice: 0.025,
  bech32prefix: 'coral',
  faucetToken: 'SHELL',
  faucetUrl: 'https://faucet.coralnet.cosmwasm.com/credit',
  hdPath: makeCosmoshubPath(0),
  defaultKeyFile: path.join(process.env.HOME, ".coral.key"),
}

interface Network {
  setup: (password: string, filename?: string) => Promise<SigningCosmWasmClient>
  recoverMnemonic: (password: string, filename?: string) => Promise<string>
}

const useOptions = (options: Options): Network => {

  const loadOrCreateWallet = async (options: Options, filename: string, password: string): Promise<Secp256k1Wallet> => {
    let encrypted: string;
    try {
      encrypted = fs.readFileSync(filename, 'utf8');
    } catch (err) {
      // generate if no file exists
      const wallet = await Secp256k1Wallet.generate(12, options.hdPath, options.bech32prefix);
      const encrypted = await wallet.serialize(password);
      fs.writeFileSync(filename, encrypted, 'utf8');
      return wallet;
    }
    // otherwise, decrypt the file (we cannot put deserialize inside try or it will over-write on a bad password)
    const wallet = await Secp256k1Wallet.deserialize(encrypted, password);
    return wallet;
  };

  const buildFeeTable = (options: Options): FeeTable => {
    const { feeToken, gasPrice } = options;
    const stdFee = (gas: number, denom: string, price: number) => {
      const amount = Math.floor(gas * price)
      return {
        amount: [{ amount: amount.toString(), denom: denom }],
        gas: gas.toString(),
      }
    }

    return {
      upload: stdFee(1500000, feeToken, gasPrice),
      init: stdFee(600000, feeToken, gasPrice),
      migrate: stdFee(600000, feeToken, gasPrice),
      exec: stdFee(200000, feeToken, gasPrice),
      send: stdFee(80000, feeToken, gasPrice),
      changeAdmin: stdFee(80000, feeToken, gasPrice),
    }
  };

  const connect = async (
    wallet: Secp256k1Wallet,
    options: Options
  ): Promise<SigningCosmWasmClient> => {
    const feeTable = buildFeeTable(options);
    const [{ address }] = await wallet.getAccounts();

    const client = new SigningCosmWasmClient(
      options.httpUrl,
      address,
      wallet,
      feeTable
    );
    return client;
  };

  const hitFaucet = async (
    faucetUrl: string,
    address: string,
    ticker: string
  ): Promise<void> => {
    await axios.post(faucetUrl, { ticker, address });
  }

  const setup = async (password: string, filename?: string): Promise<SigningCosmWasmClient> => {
    const keyfile = filename || options.defaultKeyFile;
    const wallet = await loadOrCreateWallet(coralnetOptions, keyfile, password);
    const client = await connect(wallet, coralnetOptions);

    // ensure we have some tokens
    if (options.faucetUrl) {
      const account = await client.getAccount();
      if (!account) {
        console.log(`Getting ${options.feeToken} from faucet`);
        await hitFaucet(options.faucetUrl, client.senderAddress, options.faucetToken);
      }
    }

    return client;
  }

  const recoverMnemonic = async (password: string, filename?: string): Promise<string> => {
    const keyfile = filename || options.defaultKeyFile;
    const wallet = await loadOrCreateWallet(coralnetOptions, keyfile, password);
    return wallet.mnemonic;
  }

  return {setup, recoverMnemonic};
}

type Expiration = { at_height: { height: number } } | { at_time: { time: number } } | { never: {}}

interface Bonsai {
  readonly id: string,
  readonly birth_date: number,
  readonly coin: Coin;
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

type CosmosMsg = SendMsg | DelegateMsg | UndelegateMsg | RedelegateMsg | WithdrawMsg

interface SendMsg {
  readonly bank: {
    readonly send: {
      readonly from_address: string,
      readonly to_address: string,
      readonly amount: readonly Coin[],
    }
  }
}

interface DelegateMsg {
  readonly staking: {
    readonly delegate: {
      readonly validator: string,
      readonly amount: Coin,
    }
  }
}

interface UndelegateMsg {
  readonly staking: {
    readonly undelegate: {
      readonly validator: string,
      readonly amount: Coin,
    }
  }
}

interface RedelegateMsg {
  readonly staking: {
    readonly redelegate: {
      readonly src_validator: string,
      readonly dst_validator: string,
      readonly amount: Coin,
    }
  }
}

interface WithdrawMsg {
  readonly staking: {
    readonly withdraw: {
      readonly validator: string,
      readonly recipient?: string,
    }
  }
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
  instantiate: (codeId: number, initMsg: InitMsg, label: string, admin?: string) => Promise<BonsaiInstance>

  use: (contractAddress: string) => BonsaiInstance
}

const bonsaiCW = (client: SigninfCosmWasmClient) : BonsaiContract => {
  const use = (contractAddress: string): BonsaiInstance => {
    const getBonsais = async (): Promise<BonsaiList> => {
      return client.queryContractSmart(contractAddress, {get_bonsais: {}});
    }

    const getGardener = async (address?: string): Promise<Gardener> => {
      const sender = address || client.senderAddress;
      return client.queryContractSmart(contractAddress, {get_gardener: {sender}});
    }

    const getGardeners = async () : Promise<AllGardenersResponse> => {
      return client.queryContractSmart(contractAddress, {get_gardeners: {}});
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
      source: "",
      builder: "cosmwasm/rust-optimizer:0.10.3"
    };
    const sourceUrl = "";
    const wasm = await downloadWasm(sourceUrl);
    const result = await client.upload(wasm, meta);
    return result.codeId;
  }

  const instantiate = async (codeId: number, initMsg: InitMsg, label: string, admin?: string): Promise<BonsaiInstance> => {
    const result = await client.instantiate(codeId, initMsg, label, { memo: `Init ${label}`, admin});
    return use(result.contractAddress);
  }

  return { upload, instantiate, use };

}
