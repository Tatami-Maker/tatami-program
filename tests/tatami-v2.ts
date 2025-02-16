import * as anchor from "@coral-xyz/anchor";
import { Program, AnchorProvider, Wallet } from "@coral-xyz/anchor";
import { clusterApiUrl, Connection, Keypair, PublicKey, sendAndConfirmTransaction, } from "@solana/web3.js";
import * as token from "@solana/spl-token";
import secret from "../../sol/id.json";
import {TatamiV2} from "../target/idl/tatami-v2";
import idl from "../target/idl/tatami-v2.json";
import { BN } from "bn.js";

const programId = new PublicKey("HrKLeJB6yoSWkFzVSfsg8Yi3Zs4PKZ7qqjkMz978qqZv");
const connection = new Connection(clusterApiUrl('devnet'), "confirmed");
const keypair = Keypair.fromSecretKey(Uint8Array.from(secret));
const wallet = new Wallet(keypair);
const provider = new AnchorProvider(connection, wallet, {commitment: "confirmed", maxRetries:6});
const program = new Program<TatamiV2>(idl as TatamiV2, programId, provider);
const realmProgram = new PublicKey("GovER5Lthms3bLBqWub97yVrMmEogzX7xNjdXpPPCVZw");
const metadataProgram = new PublicKey("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

describe("tatami-program", () => {
    const daoName = "TestTestTest111";

    const [config] = PublicKey.findProgramAddressSync([Buffer.from("tatami-config")], programId);
    const [vault] = PublicKey.findProgramAddressSync([Buffer.from("tatami-vault")], programId);

    // const mint = Keypair.generate();
    const mint = {
        publicKey: new PublicKey("32vzj4fHpxwV3FnTDfmh1n6LiP66oqpKkr2rnrL1W3xT"),
        secretKey: Keypair.generate().secretKey
    }
    const councilMint = Keypair.generate();

    const [metadata] = PublicKey.findProgramAddressSync([
        Buffer.from("metadata"),
        metadataProgram.toBuffer(),
        mint.publicKey.toBuffer(),
    ], 
        metadataProgram
    );

    const [project] = PublicKey.findProgramAddressSync([
        Buffer.from("tatami-project"),
        mint.publicKey.toBuffer()
    ], programId);

    const [realmAccount] = PublicKey.findProgramAddressSync([
        Buffer.from("governance"),
        Buffer.from(daoName)
    ], realmProgram);

    const [communityTokenHolding] = PublicKey.findProgramAddressSync([
        Buffer.from("governance"),
        realmAccount.toBytes(),
        mint.publicKey.toBytes()
    ], realmProgram);

    const [councilTokenHolding] = PublicKey.findProgramAddressSync([
        Buffer.from("governance"),
        realmAccount.toBytes(),
        councilMint.publicKey.toBytes()
    ], realmProgram);

    const [realmConfig] = PublicKey.findProgramAddressSync([
        Buffer.from('realm-config'),
        realmAccount.toBytes()
    ], realmProgram);

    const governedAccount = Keypair.generate().publicKey;

    const [governance] = PublicKey.findProgramAddressSync([
        Buffer.from("account-governance"),
        realmAccount.toBytes(),
        governedAccount.toBytes()
    ], realmProgram);

    const [nativeTreasury] = PublicKey.findProgramAddressSync([
        Buffer.from("native-treasury"),
        governance.toBytes()
    ], realmProgram);

    const teamWallet = keypair.publicKey;
    const teamTokenAccount = token.getAssociatedTokenAddressSync(mint.publicKey, teamWallet, true);
    const vaultTokenAccount = token.getAssociatedTokenAddressSync(mint.publicKey, vault, true);
    const daoTokenAccount = token.getAssociatedTokenAddressSync(mint.publicKey, nativeTreasury, true);

    xit("creates config", async() => {
        const tx = await program.methods.createConfig(new BN(0))
        .accounts({config}).rpc();

        console.log("Tx: ", tx);

        const configDetails = await program.account.config.fetch(config);
        console.log(configDetails);
    });

    xit("initializes project and create token and DAO", async() => {
        const tx = await program.methods.initProject(6, "Tatami Coin", "TTM", "", 560, [new BN(568500000), new BN(789562000)])
        .accounts({
            config,
            project,
            mint: mint.publicKey,
            metadata,
            metadataProgram,
            vault,
            teamWallet,
            teamTokenAccount,
            vaultTokenAccount
        })
        .signers([mint])
        .transaction()

        const initDaoIx = await program.methods.initializeDao(daoName, new BN(5000000), new BN(1000000), 
        false, 5, 86400)
        .accounts({
            mint: mint.publicKey,
            councilMint: null,
            communityTokenHolding,
            realmAccount,
            realmConfig,
            realmProgram,
            councilTokenHolding: null,
            governance,
            governedAccount,
            nativeTreasury,
            project,
            daoTokenAccount,
        })
        .instruction();

        tx.add(initDaoIx);

        try {
            const sig = await sendAndConfirmTransaction(connection, tx, [keypair, mint]);
            console.log(sig)
        } catch(e) {
            console.log(e)
        }
    });

    it("airdrops tokens",async () => {
        const receiver = new PublicKey("5XF5SvWVo7TPMEpVqZgYRdwKXAMVC41CdBquDZcATNtJ");
        const recipientTokenAccount = anchor.utils.token.associatedAddress({mint: mint.publicKey, owner: receiver});

        const tx = await program.methods.airdropTokens(new BN(5000000))
        .accounts({
            project,
            vault,
            vaultTokenAccount,
            receiver,
            recipientTokenAccount,
            mint: mint.publicKey
        })    
        .rpc();

        console.log(tx);
    })

    xit("fetches project account", async() => {
        const [key] = PublicKey.findProgramAddressSync([
            Buffer.from("governance"),
            Buffer.from("DAO DAOD")
        ], new PublicKey("GovER5Lthms3bLBqWub97yVrMmEogzX7xNjdXpPPCVZw"))
        
        console.log(key.toBase58())
        // const account = await program.account.project.fetch("9PqNUaD7JuTEtUYzs5wxEDm7kbGwPdAa2TabiwtLxxwg");
        // console.log(account)
    })
});