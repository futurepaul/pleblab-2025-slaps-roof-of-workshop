import { useState, useEffect } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const [heartbeatCount, setHeartbeatCount] = useState(0);
  const [lastPingResponse, setLastPingResponse] = useState<string | null>(null);
  const [lastDataUpdate, setLastDataUpdate] = useState<string | null>(null);
  
  // Wallet state
  const [walletAddress, setWalletAddress] = useState<string | null>(null);
  const [walletBalance, setWalletBalance] = useState<number | null>(null);
  const [syncStatus, setSyncStatus] = useState<string | null>(null);
  const [txid, setTxid] = useState<string | null>(null);
  const [walletError, setWalletError] = useState<string | null>(null);
  const [sendAmount, setSendAmount] = useState<number>(5000);

  useEffect(() => {
    const unlistenBackgroundEvent = listen("background-event", (event) => {
      console.log("Received background event:", event);
      setLastPingResponse(event.payload as string);
    });

    const unlistenDataUpdated = listen("data-updated", (event) => {
      console.log("Data updated:", event);
      setLastDataUpdate(event.payload as string);
    });

    const unlistenHeartbeat = listen("heartbeat", (event) => {
      console.log("Heartbeat received from background task:", event);
      setHeartbeatCount(event.payload as number);
    });
    
    // Wallet event listeners
    const unlistenWalletAddress = listen("wallet-address", (event) => {
      console.log("Wallet address received:", event);
      setWalletAddress(event.payload as string);
    });
    
    const unlistenWalletBalance = listen("wallet-balance", (event) => {
      console.log("Wallet balance received:", event);
      setWalletBalance(parseInt(event.payload as string));
    });
    
    const unlistenSyncStarted = listen("sync-started", (event) => {
      console.log("Wallet sync started:", event);
      setSyncStatus("Syncing...");
    });
    
    const unlistenSyncProgress = listen("sync-progress", (event) => {
      console.log("Wallet sync progress:", event);
      setSyncStatus(event.payload as string);
    });
    
    const unlistenSyncCompleted = listen("sync-completed", (event) => {
      console.log("Wallet sync completed:", event);
      setSyncStatus("Sync completed");
      setWalletBalance(parseInt(event.payload as string));
    });
    
    const unlistenTransactionSent = listen("transaction-sent", (event) => {
      console.log("Transaction sent:", event);
      setTxid(event.payload as string);
    });
    
    const unlistenWalletError = listen("wallet-error", (event) => {
      console.log("Wallet error:", event);
      setWalletError(event.payload as string);
      // Clear error after 5 seconds
      setTimeout(() => setWalletError(null), 5000);
    });

    return () => {
      unlistenBackgroundEvent.then(unsub => unsub());
      unlistenDataUpdated.then(unsub => unsub());
      unlistenHeartbeat.then(unsub => unsub());
      unlistenWalletAddress.then(unsub => unsub());
      unlistenWalletBalance.then(unsub => unsub());
      unlistenSyncStarted.then(unsub => unsub());
      unlistenSyncProgress.then(unsub => unsub());
      unlistenSyncCompleted.then(unsub => unsub());
      unlistenTransactionSent.then(unsub => unsub());
      unlistenWalletError.then(unsub => unsub());
    };
  }, []);

  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    setGreetMsg(await invoke("greet", { name: name + name }));
  }

  const sendPing = async () => {
    try {
      await invoke("send_to_background", {
        message: { Ping: null }
      });
      console.log("Ping sent to background task");
    } catch (error) {
      console.error("Error sending ping:", error);
    }
  };

  const sendData = async (data: string) => {
    try {
      await invoke("send_to_background", {
        message: { UpdateData: data }
      });
      console.log("Data sent to background task");
    } catch (error) {
      console.error("Error sending data:", error);
    }
  };
  
  // Wallet functions
  const getWalletAddress = async () => {
    try {
      await invoke("send_to_background", {
        message: { GetWalletAddress: null }
      });
      console.log("Get wallet address request sent");
    } catch (error) {
      console.error("Error requesting wallet address:", error);
    }
  };
  
  const syncWallet = async () => {
    try {
      await invoke("send_to_background", {
        message: { SyncWallet: null }
      });
      console.log("Sync wallet request sent");
    } catch (error) {
      console.error("Error requesting wallet sync:", error);
    }
  };
  
  const getWalletBalance = async () => {
    try {
      await invoke("send_to_background", {
        message: { GetWalletBalance: null }
      });
      console.log("Get wallet balance request sent");
    } catch (error) {
      console.error("Error requesting wallet balance:", error);
    }
  };
  
  const sendTransaction = async () => {
    try {
      await invoke("send_to_background", {
        message: { SendTransaction: sendAmount }
      });
      console.log("Send transaction request sent");
    } catch (error) {
      console.error("Error requesting transaction send:", error);
    }
  };

  return (
    <main className="container">
      <h1>BDK Wallet with Tauri</h1>

      <div className="row">
        <a href="https://vitejs.dev" target="_blank">
          <img src="/vite.svg" className="logo vite" alt="Vite logo" />
        </a>
        <a href="https://tauri.app" target="_blank">
          <img src="/tauri.svg" className="logo tauri" alt="Tauri logo" />
        </a>
        <a href="https://reactjs.org" target="_blank">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </a>
      </div>

      <div className="card">
        <h2>Bitcoin Development Kit Wallet</h2>
        
        {walletError && (
          <div className="error-box">
            <strong>Error:</strong> {walletError}
          </div>
        )}
        
        <div className="button-row">
          <button onClick={getWalletAddress}>Create/Get Address</button>
          <button onClick={syncWallet}>Sync Wallet</button>
          <button onClick={getWalletBalance}>Get Balance</button>
        </div>
        
        {walletAddress && (
          <div className="info-box">
            <strong>Wallet Address:</strong>
            <p className="address">{walletAddress.split('|')[1]}</p>
            <p><small>Index: {walletAddress.split('|')[0]}</small></p>
          </div>
        )}
        
        {walletBalance !== null && (
          <div className="info-box">
            <strong>Wallet Balance:</strong>
            <p>{walletBalance} sats</p>
          </div>
        )}
        
        {syncStatus && (
          <div className="info-box">
            <strong>Sync Status:</strong>
            <p>{syncStatus}</p>
          </div>
        )}
        
        <div className="transaction-box">
          <h3>Send Transaction</h3>
          <div className="input-row">
            <input
              type="number"
              value={sendAmount}
              onChange={(e) => setSendAmount(parseInt(e.target.value))}
              placeholder="Amount in sats"
              min="1000"
            />
            <button onClick={sendTransaction}>Send</button>
          </div>
          
          {txid && (
            <div className="info-box">
              <strong>Transaction Sent:</strong>
              <p className="txid">{txid}</p>
            </div>
          )}
        </div>
      </div>

      <div className="card">
        <h2>Background Task Communication</h2>
        <div className="stat-box">
          <strong>Heartbeat Count:</strong> {heartbeatCount}
          <p><small>(Updates every 10 seconds from the background task)</small></p>
        </div>
        
        <button onClick={sendPing}>Send Ping</button>
        {lastPingResponse && (
          <div className="response-box">
            <strong>Last Ping Response:</strong> {lastPingResponse}
          </div>
        )}
        
        <button onClick={() => sendData("Hello from frontend!")}>Send Data</button>
        {lastDataUpdate && (
          <div className="response-box">
            <strong>Last Data Update:</strong> {lastDataUpdate}
          </div>
        )}
      </div>
      
      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          greet();
        }}
      >
        <input
          id="greet-input"
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="Enter a name..."
        />
        <button type="submit">Greet</button>
      </form>
      <p>{greetMsg}</p>
    </main>
  );
}

export default App;
