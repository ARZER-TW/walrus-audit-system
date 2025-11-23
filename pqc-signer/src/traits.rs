/// 後量子簽名統一接口
use crate::error::Result;

/// 簽名者特徵
pub trait Signer {
    /// 生成密鑰對
    fn generate_keypair(&mut self) -> Result<()>;

    /// 對消息簽名
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>>;

    /// 驗證簽名
    fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool>;

    /// 獲取公鑰
    fn public_key(&self) -> &[u8];

    /// 算法名稱
    fn algorithm_name(&self) -> &str;
}
