use std::rc::Rc;
use crate::wallet::ledger::ledger::LedgerWallet;
use crate::wallet::keystone::keystone::KeystoneWallet;
use crate::remote_wallet::RemoteWalletInfo;

#[derive(Debug)]
pub struct Device {
    pub(crate) path: String,
    pub(crate) info: RemoteWalletInfo,
    pub wallet_type: RemoteWalletType,
}

#[derive(Debug)]
pub enum RemoteWalletType {
    Ledger(Rc<LedgerWallet>),
    Keystone(Rc<KeystoneWallet>),
}