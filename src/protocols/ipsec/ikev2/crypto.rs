// src/protocols/ipsec/ikev2/crypto.rs
//
// IKEv2 加密和密钥派生（简化教学实现）

use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

// ========== PRF 函数（简化实现） ==========

/// IKEv2 伪随机函数
///
/// RFC 7296 Section 2.13:
/// prf+ (Key, Data) = T1 | T2 | T3 | T4 | ...
/// 其中:
/// T1 = prf (Key, Data | 0x01)
/// T2 = prf (Key, T1 | Data | 0x02)
/// T3 = prf (Key, T2 | Data | 0x03)
/// ...
///
/// 注意：这是简化实现，使用基本的哈希操作
/// 实际应用应使用标准加密库
pub struct IkePseudoRandomFunction;

impl IkePseudoRandomFunction {
    /// 简化的 PRF 实现
    ///
    /// # 参数
    /// - `key`: 密钥
    /// - `data`: 数据
    /// - `output_len`: 输出长度
    pub fn prf_plus(key: &[u8], data: &[u8], output_len: usize) -> Vec<u8> {
        let mut result = Vec::with_capacity(output_len);
        let mut previous = Vec::new();

        let mut round = 1u8;
        while result.len() < output_len {
            // T_i = prf(Key, T_{i-1} | Data | i)
            let mut input = Vec::new();
            input.extend_from_slice(&previous);
            input.extend_from_slice(data);
            input.push(round);

            let t_i = Self::simple_prf(key, &input);
            previous = t_i.clone();
            result.extend_from_slice(&t_i);

            round += 1;
        }

        result.truncate(output_len);
        result
    }

    /// 简化的 PRF 基础函数
    ///
    /// 实际应用应使用 HMAC-SHA256 等标准算法
    fn simple_prf(key: &[u8], data: &[u8]) -> Vec<u8> {
        // 简化实现：混合密钥和数据
        let mut result = Vec::with_capacity(32);

        let key_len = key.len();
        let data_len = data.len();

        for i in 0..32 {
            let key_byte = key[i % key_len];
            let data_byte = if data_len > 0 { data[i % data_len] } else { 0 };
            let round_byte = i as u8;

            let output = (key_byte.wrapping_mul(3)
                .wrapping_add(data_byte)
                ^ round_byte)
                .wrapping_mul(7);
            result.push(output);
        }

        result
    }

    /// 计算 SKEYSEED
    ///
    /// RFC 7296: SKEYSEED = prf(Ni | Nr, g^ir)
    pub fn compute_skeyseed(ni: &[u8], nr: &[u8], dh_shared: &[u8]) -> Vec<u8> {
        let mut input = Vec::new();
        input.extend_from_slice(ni);
        input.extend_from_slice(nr);

        Self::simple_prf(&input, dh_shared)
    }

    /// 派生密钥材料
    ///
    /// RFC 7296 Section 2.14:
    /// KEYMAT = prf+ (SKEYSEED, Ni | Nr | SPIi | SPIr)
    pub fn derive_keymat(
        skeyseed: &[u8],
        ni: &[u8],
        nr: &[u8],
        spi_i: &[u8; SPI_LEN],
        spi_r: &[u8; SPI_LEN],
        keymat_len: usize,
    ) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(ni);
        data.extend_from_slice(nr);
        data.extend_from_slice(spi_i);
        data.extend_from_slice(spi_r);

        Self::prf_plus(skeyseed, &data, keymat_len)
    }
}

// ========== DH 计算（简化实现） ==========

/// 计算 DH 共享密钥
///
/// 注意：这是简化实现，不提供真实的 DH 安全性
/// 实际应用应使用标准加密库（如 RustCrypto 的 p256、k256 等）
pub fn compute_dh_shared(_dh_group: IkeDhGroup, public_key: &[u8], private_key: &[u8]) -> IkeResult<Vec<u8>> {
    // 简化实现：使用 XOR 操作模拟 DH 共享密钥计算
    let mut shared = Vec::new();
    let key_len = private_key.len().min(public_key.len());

    for i in 0..key_len {
        shared.push(public_key[i] ^ private_key[i]);
    }

    // 填充到至少 32 字节
    while shared.len() < 32 {
        shared.push(0x00);
    }

    Ok(shared)
}

/// 生成 DH 密钥对
///
/// 注意：这是简化实现，不提供真实的 DH 安全性
pub fn generate_dh_keypair(_dh_group: IkeDhGroup) -> IkeResult<(Vec<u8>, Vec<u8>)> {
    // 简化实现：返回模拟的公私钥对
    // 实际应用应使用标准加密库生成真实的 DH 密钥对

    let mut private_key = vec![0u8; 32];
    let mut public_key = vec![0u8; 32];

    // 使用模拟随机值
    for i in 0..32 {
        private_key[i] = ((i as u32).wrapping_mul(0x9E3779B9) >> 8) as u8;
        public_key[i] = private_key[i].wrapping_add(0x13);
    }

    Ok((private_key, public_key))
}

// ========== 随机数生成 ==========

/// 生成随机 SPI
pub fn generate_random_spi() -> [u8; SPI_LEN] {
    let mut spi = [0u8; SPI_LEN];
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    let mut seed = timestamp.wrapping_mul(0x9E3779B9);
    for i in 0..SPI_LEN {
        seed = seed.wrapping_mul(0x9E3779B9).wrapping_add(0x243F6A88);
        spi[i] = (seed >> 24) as u8;
    }

    // 确保 SPI 不为全 0
    if spi == [0u8; SPI_LEN] {
        spi[0] = 0x01;
    }

    spi
}

/// 生成随机 Nonce
pub fn generate_random_nonce(size: usize) -> Vec<u8> {
    let mut nonce = vec![0u8; size];
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;

    let mut seed = timestamp.wrapping_mul(0x9E3779B9);
    for i in 0..size {
        seed = seed.wrapping_mul(0x9E3779B9).wrapping_add(0x243F6A88);
        nonce[i] = (seed >> 24) as u8;
    }

    nonce
}

// ========== 密钥材料计算 ==========

/// 计算完整的密钥材料
///
/// # 参数
/// - `ni`: 发起方 Nonce
/// - `nr`: 响应方 Nonce
/// - `dh_shared`: DH 共享密钥
/// - `spi_i`: 发起方 SPI
/// - `spi_r`: 响应方 SPI
/// - `enc_key_len`: 加密密钥长度
/// - `auth_key_len`: 认证密钥长度
pub fn compute_key_material(
    ni: &[u8],
    nr: &[u8],
    dh_shared: &[u8],
    spi_i: &[u8; SPI_LEN],
    spi_r: &[u8; SPI_LEN],
    enc_key_len: usize,
    auth_key_len: usize,
) -> IkeResult<IkeKeyMaterial> {
    // 计算 SKEYSEED
    let skeyseed = IkePseudoRandomFunction::compute_skeyseed(ni, nr, dh_shared);

    // 计算所需的密钥材料总长度
    // SK_d (固定) + SK_ai + SK_ar + SK_ei + SK_er + SK_pi + SK_pr
    let sk_d_len = 32; // 固定 32 字节
    let total_keymat_len = sk_d_len + auth_key_len * 2 + enc_key_len * 2 + auth_key_len * 2;

    // 派生密钥材料
    let keymat = IkePseudoRandomFunction::derive_keymat(
        &skeyseed,
        ni,
        nr,
        spi_i,
        spi_r,
        total_keymat_len,
    );

    // 解析密钥材料
    let mut offset = 0;

    let sk_d = keymat[offset..offset + sk_d_len].to_vec();
    offset += sk_d_len;

    let sk_ai = keymat[offset..offset + auth_key_len].to_vec();
    offset += auth_key_len;

    let sk_ar = keymat[offset..offset + auth_key_len].to_vec();
    offset += auth_key_len;

    let sk_ei = keymat[offset..offset + enc_key_len].to_vec();
    offset += enc_key_len;

    let sk_er = keymat[offset..offset + enc_key_len].to_vec();
    offset += enc_key_len;

    let sk_pi = keymat[offset..offset + auth_key_len].to_vec();
    offset += auth_key_len;

    let sk_pr = keymat[offset..offset + auth_key_len].to_vec();

    Ok(IkeKeyMaterial::new(
        sk_d,
        sk_ai,
        sk_ar,
        sk_ei,
        sk_er,
        sk_pi,
        sk_pr,
    ))
}

// ========== 加密/解密（简化实现） ==========

/// IKE 加密操作
pub struct IkeCrypto;

impl IkeCrypto {
    /// 加密数据
    ///
    /// 注意：这是简化实现，使用 XOR 混淆
    /// 实际应用应使用 AES-GCM 等标准算法
    pub fn encrypt(key: &[u8], plaintext: &[u8], iv: &[u8]) -> Vec<u8> {
        let mut ciphertext = Vec::with_capacity(plaintext.len());

        let key_len = key.len();
        let iv_len = iv.len();

        for (i, &byte) in plaintext.iter().enumerate() {
            let key_byte = key[i % key_len];
            let iv_byte = iv[i % iv_len];
            let obfuscate = ((i as u8).wrapping_mul(17)).wrapping_add(73);
            ciphertext.push(byte ^ key_byte ^ iv_byte ^ obfuscate);
        }

        ciphertext
    }

    /// 解密数据
    ///
    /// 与 encrypt 使用相同的操作（异或自反）
    pub fn decrypt(key: &[u8], ciphertext: &[u8], iv: &[u8]) -> Vec<u8> {
        Self::encrypt(key, ciphertext, iv)
    }

    /// 生成 IV
    pub fn generate_iv(length: usize) -> Vec<u8> {
        generate_random_nonce(length)
    }

    /// 计算 MAC（简化实现）
    ///
    /// 注意：这是简化实现
    /// 实际应用应使用 HMAC-SHA256 等标准算法
    pub fn compute_mac(key: &[u8], data: &[u8], output_len: usize) -> Vec<u8> {
        let mut mac = Vec::with_capacity(output_len);

        let key_len = key.len();
        let data_len = data.len();

        for i in 0..output_len {
            let key_byte = key[i % key_len];
            let data_byte = if data_len > 0 { data[i % data_len] } else { 0 };
            let round_byte = i as u8;

            let output = (key_byte.wrapping_mul(3)
                .wrapping_add(data_byte)
                ^ round_byte)
                .wrapping_mul(7)
                .rotate_left(3);
            mac.push(output);
        }

        mac
    }

    /// 验证 MAC
    pub fn verify_mac(key: &[u8], data: &[u8], expected: &[u8]) -> bool {
        let computed = Self::compute_mac(key, data, expected.len());
        super::super::constant_time_compare(&computed, expected)
    }

    /// 计算 AUTH payload 数据
    ///
    /// RFC 7296 Section 2.15:
    /// AUTH = prf(prf(Shared Secret, "Key Pad for IKEv2"), <MsgOctets>)
    pub fn compute_auth_data(
        shared_secret: &[u8],
        msg_octets: &[u8],
        output_len: usize,
    ) -> Vec<u8> {
        // Key Pad for IKEv2 (RFC 7296)
        const KEY_PAD: &[u8] = b"Key Pad for IKEv2";

        // prf(Shared Secret, "Key Pad for IKEv2")
        let key = IkePseudoRandomFunction::simple_prf(shared_secret, KEY_PAD);

        // prf(key, <MsgOctets>)
        IkePseudoRandomFunction::simple_prf(&key, msg_octets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prf_plus() {
        let key = vec![0x01, 0x02, 0x03, 0x04];
        let data = vec![0xAA, 0xBB, 0xCC];
        let result = IkePseudoRandomFunction::prf_plus(&key, &data, 64);

        assert_eq!(result.len(), 64);
        // 验证结果不是全零
        assert!(!result.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_compute_skeyseed() {
        let ni = vec![0x01; 32];
        let nr = vec![0x02; 32];
        let dh_shared = vec![0x03; 32];

        let skeyseed = IkePseudoRandomFunction::compute_skeyseed(&ni, &nr, &dh_shared);

        assert_eq!(skeyseed.len(), 32);
        assert!(!skeyseed.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_compute_key_material() {
        let ni = vec![0x01; 32];
        let nr = vec![0x02; 32];
        let dh_shared = vec![0x03; 32];
        let spi_i = [0x11; 8];
        let spi_r = [0x22; 8];

        let keymat = compute_key_material(&ni, &nr, &dh_shared, &spi_i, &spi_r, 32, 32).unwrap();

        assert_eq!(keymat.sk_d.len(), 32);
        assert_eq!(keymat.sk_ai.len(), 32);
        assert_eq!(keymat.sk_ar.len(), 32);
        assert_eq!(keymat.sk_ei.len(), 32);
        assert_eq!(keymat.sk_er.len(), 32);
        assert_eq!(keymat.sk_pi.len(), 32);
        assert_eq!(keymat.sk_pr.len(), 32);
    }

    #[test]
    fn test_dh_keypair() {
        let (private, public) = generate_dh_keypair(IkeDhGroup::MODP2048).unwrap();

        assert_eq!(private.len(), 32);
        assert_eq!(public.len(), 32);
    }

    #[test]
    fn test_dh_shared() {
        let (private1, public1) = generate_dh_keypair(IkeDhGroup::MODP2048).unwrap();
        let (_, public2) = generate_dh_keypair(IkeDhGroup::MODP2048).unwrap();

        let shared1 = compute_dh_shared(IkeDhGroup::MODP2048, &public2, &private1).unwrap();
        let shared2 = compute_dh_shared(IkeDhGroup::MODP2048, &public1, &private1).unwrap();

        assert_eq!(shared1.len(), 32);
        assert_eq!(shared2.len(), 32);
    }

    #[test]
    fn test_random_spi() {
        let spi1 = generate_random_spi();
        let spi2 = generate_random_spi();

        assert_ne!(spi1, [0u8; 8]);
        assert_ne!(spi2, [0u8; 8]);
        assert_ne!(spi1, spi2);
    }

    #[test]
    fn test_random_nonce() {
        let nonce1 = generate_random_nonce(32);
        let nonce2 = generate_random_nonce(32);

        assert_eq!(nonce1.len(), 32);
        assert_eq!(nonce2.len(), 32);
        assert_ne!(nonce1, nonce2);
    }

    #[test]
    fn test_encrypt_decrypt() {
        let key = vec![0x01, 0x02, 0x03, 0x04];
        let iv = vec![0x05, 0x06, 0x07, 0x08];
        let plaintext = b"Hello, World!".to_vec();

        let ciphertext = IkeCrypto::encrypt(&key, &plaintext, &iv);
        let decrypted = IkeCrypto::decrypt(&key, &ciphertext, &iv);

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_compute_mac() {
        let key = vec![0x01, 0x02, 0x03, 0x04];
        let data = vec![0xAA, 0xBB, 0xCC];

        let mac1 = IkeCrypto::compute_mac(&key, &data, 16);
        let mac2 = IkeCrypto::compute_mac(&key, &data, 16);

        assert_eq!(mac1, mac2);
        assert!(IkeCrypto::verify_mac(&key, &data, &mac1));
    }

    #[test]
    fn test_compute_auth_data() {
        let shared_secret = vec![0x01; 32];
        let msg_octets = vec![0x02; 64];

        let auth_data = IkeCrypto::compute_auth_data(&shared_secret, &msg_octets, 32);

        assert_eq!(auth_data.len(), 32);
    }
}
