#![no_std] // Rust projesi standart kütüphaneyi (std) kullanmayacağını belirten satır

// Bu satır, soroban_sdk kütüphanesinden gerekli modülleri ve tipleri içe aktarır.
// - `contract`: Bir struct'ı Soroban kontratı olarak işaretlemek için kullanılır.
// - `contractimpl`: Bir struct için kontrat fonksiyonlarını (arayüzünü) implemente etmek için kullanılır.
// - `contracttype`: Bir enum veya struct'ı kontrat depolaması ve arayüzleri için uygun hale getirmek için kullanılır (serileştirme/deserileştirme).
// - `token`: Standart token kontratlarıyla etkileşim kurmak için bir istemci sağlar.
// - `Address`: Bir Soroban hesabını veya kontratını temsil eden bir adres tipidir.
// - `Env`: Kontratın çalıştığı Soroban ortamına erişim sağlar (örn: depolama, defter bilgisi, vb.).
// - `Vec`: Dinamik boyutlu bir vektör (liste) tipidir.
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env, Vec};

// `#[derive(Clone)]`: Bu enum'un kopyalanabilir (cloneable) olmasını sağlar.
// `#[contracttype]`: Bu enum'un kontrat depolamasında anahtar veya değer olarak kullanılabilmesi için
// Soroban tarafından serileştirilebilir/deserileştirilebilir olmasını sağlar.
#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    // Kontratın başlatılıp başlatılmadığını kontrol etmek için kullanılacak bir depolama anahtarı.
    Init,
    // Talep edilebilir bakiyenin bilgilerini saklamak için kullanılacak bir depolama anahtarı.
    Balance,
}

// `#[derive(Clone)]` ve `#[contracttype]` yukarıdaki `DataKey` ile aynı amaçla kullanılır.
#[derive(Clone)]
#[contracttype]
pub enum TimeBoundKind {
    // Zaman sınırının, belirtilen bir zamandan "önce" olduğunu ifade eder.
    Before,
    // Zaman sınırının, belirtilen bir zamandan "sonra" olduğunu ifade eder.
    After,
}

// `#[derive(Clone)]` ve `#[contracttype]` yukarıdaki `DataKey` ile aynı amaçla kullanılır.
#[derive(Clone)]
#[contracttype]
pub struct TimeBound {
    // Zaman sınırının türünü (Before veya After) tutar.
    pub kind: TimeBoundKind,
    // Zaman sınırının karşılaştırılacağı Unix zaman damgasını (saniye cinsinden) tutar.
    pub timestamp: u64,
}

// `#[derive(Clone)]` ve `#[contracttype]` yukarıdaki `DataKey` ile aynı amaçla kullanılır.
#[derive(Clone)]
#[contracttype]
pub struct ClaimableBalance {
    // Talep edilebilir bakiyenin token türünün adresi.
    pub token: Address,
    // Talep edilebilir token miktarı.
    pub amount: i128,
    // Bu bakiyeyi talep etme hakkına sahip olan adreslerin bir listesi.
    pub claimants: Vec<Address>,
    // Bakiyenin ne zaman talep edilebileceğini belirleyen zaman sınırı.
    pub time_bound: TimeBound,
}

// `#[contract]` makrosu, bu struct'ı bir Soroban akıllı kontratı olarak tanımlar.
// `ClaimableBalanceContract` struct'ı, kontratın durumunu (state) tutmak için kullanılabilir,
// ancak bu örnekte durum, `env.storage()` aracılığıyla yönetildiği için boştur.
#[contract]
pub struct ClaimableBalanceContract;

// 'timelock' kısmı: Sağlanan zaman damgasının mevcut defter zaman damgasından
// önce/sonra olup olmadığını kontrol eder.
// `env`: Kontrat ortamına erişim.
// `time_bound`: Kontrol edilecek zaman sınırı bilgisi.
// `-> bool`: Fonksiyonun bir boolean (true/false) değer döndüreceğini belirtir.
fn check_time_bound(env: &Env, time_bound: &TimeBound) -> bool {
    // Mevcut defterin (ledger) zaman damgasını alır. Bu, blok zincirinin mevcut zamanıdır.
    let ledger_timestamp = env.ledger().timestamp();

    // `time_bound.kind`'e göre eşleşme yapar.
    match time_bound.kind {
        // Eğer zaman sınırı türü 'Before' ise:
        // Defter zaman damgası, belirtilen zaman damgasından küçük veya eşitse true döner.
        // Yani, "belirtilen zamandan önce" koşulu sağlanmış olur.
        TimeBoundKind::Before => ledger_timestamp <= time_bound.timestamp,
        // Eğer zaman sınırı türü 'After' ise:
        // Defter zaman damgası, belirtilen zaman damgasından büyük veya eşitse true döner.
        // Yani, "belirtilen zamandan sonra" koşulu sağlanmış olur.
        TimeBoundKind::After => ledger_timestamp >= time_bound.timestamp,
    }
}

// `#[contractimpl]` makrosu, `ClaimableBalanceContract` için kontrat fonksiyonlarını implemente eder.
// Bu blok içindeki public fonksiyonlar, kontratın dışarıdan çağrılabilir arayüzünü oluşturur.
#[contractimpl]
impl ClaimableBalanceContract {
    // `deposit` fonksiyonu, bir kullanıcı tarafından token yatırılmasına ve talep edilebilir bir bakiye oluşturulmasına olanak tanır.
    // `env`: Kontrat ortamı.
    // `from`: Token'ları yatıran hesabın adresi.
    // `token`: Yatırılan token'ın kontrat adresi.
    // `amount`: Yatırılan token miktarı.
    // `claimants`: Bakiyeyi talep edebilecek adreslerin listesi.
    // `time_bound`: Bakiyenin ne zaman talep edilebileceğini belirleyen zaman sınırı.
    pub fn deposit(
        env: Env,
        from: Address,
        token: Address,
        amount: i128,
        claimants: Vec<Address>,
        time_bound: TimeBound,
    ) {
        // Talep edenlerin sayısı 10'dan fazlaysa, işlemi paniklet (durdur ve geri al).
        // Bu, kontratın aşırı büyük bir talepçi listesiyle şişirilmesini önler.
        if claimants.len() > 10 {
            panic!("too many claimants"); // "çok fazla talepçi"
        }
        // Eğer kontrat `is_initialized` fonksiyonuna göre zaten başlatılmışsa, paniklet.
        // Bu kontratın sadece bir kez (tek bir talep edilebilir bakiye için) kullanılması amaçlanmıştır.
        if is_initialized(&env) {
            panic!("contract has been already initialized"); // "kontrat zaten başlatılmış"
        }
        // `from` adresinin bu `deposit` çağrısını tüm argümanlarla birlikte yetkilendirdiğinden emin ol.
        // Bu, `from` hesabının işlemi onayladığını garanti eder.
        from.require_auth();

        // Token'ları `from` adresinden bu kontratın adresine transfer et.
        // `token::Client::new` ile belirtilen token kontratı için bir istemci oluşturulur.
        // `.transfer` fonksiyonu ile `from`'dan `env.current_contract_address()`'a (bu kontratın adresi) `amount` kadar token transfer edilir.
        token::Client::new(&env, &token).transfer(&from, &env.current_contract_address(), &amount);
        // Talep edenlerden birinin bakiyeyi talep etmesine izin vermek için gerekli tüm bilgileri sakla.
        // `env.storage().instance()` ile kontratın örnek depolamasına erişilir.
        // `.set` ile `DataKey::Balance` anahtarı altına `ClaimableBalance` struct'ı kaydedilir.
        env.storage().instance().set(
            &DataKey::Balance, // Anahtar olarak DataKey::Balance kullanılır.
            &ClaimableBalance { // Değer olarak ClaimableBalance struct'ı kullanılır.
                token,          // Token adresi.
                amount,         // Token miktarı.
                time_bound,     // Zaman sınırı.
                claimants,      // Talepçiler listesi.
            },
        );
        // Kontratı başlatılmış olarak işaretle, tekrar kullanılmasını önlemek için.
        // Bu, başlatmayı ele almanın sadece bir yoludur - bir kontratın birden fazla
        // talep edilebilir bakiyeyi yönetmesine izin vermek de mümkün olabilir.
        // `DataKey::Init` anahtarı altına boş bir tuple `()` kaydedilerek kontratın başlatıldığı işaretlenir.
        env.storage().instance().set(&DataKey::Init, &());
    }

    // `claim` fonksiyonu, yetkili bir talepçinin depolanmış bakiyeyi talep etmesine olanak tanır.
    // `env`: Kontrat ortamı.
    // `claimant`: Bakiyeyi talep eden hesabın adresi.
    pub fn claim(env: Env, claimant: Address) {
        // Talepçinin bu çağrıyı yetkilendirdiğinden emin ol, bu da kimliklerini doğrular.
        claimant.require_auth();
        // Sadece bakiyeyi al - eğer talep edilmişse (yani `DataKey::Balance` silinmişse),
        // `.get()` fonksiyonundan sonraki `.unwrap()` panikleyerek kontratın çalışmasını sonlandıracaktır.
        let claimable_balance: ClaimableBalance =
            env.storage().instance().get(&DataKey::Balance).unwrap(); // `unwrap` ile Option<ClaimableBalance> içinden değeri alır veya panikler.

        // `check_time_bound` fonksiyonunu kullanarak zaman koşulunun karşılanıp karşılanmadığını kontrol et.
        if !check_time_bound(&env, &claimable_balance.time_bound) {
            panic!("time predicate is not fulfilled"); // "zaman koşulu karşılanmadı"
        }

        // Talep edilebilir bakiyedeki talepçi listesini al.
        let claimants = &claimable_balance.claimants;
        // Çağrıyı yapan `claimant`'ın, izin verilen talepçiler listesinde olup olmadığını kontrol et.
        if !claimants.contains(&claimant) {
            panic!("claimant is not allowed to claim this balance"); // "talepçi bu bakiyeyi talep etmeye yetkili değil"
        }

        // Tüm kontroller geçildikten sonra, saklanan miktardaki token'ı talepçiye transfer et.
        // Token'lar bu kontratın adresinden (`env.current_contract_address()`) `claimant`'a transfer edilir.
        token::Client::new(&env, &claimable_balance.token).transfer(
            &env.current_contract_address(),
            &claimant,
            &claimable_balance.amount,
        );
        // Daha fazla talep yapılmasını önlemek için bakiye kaydını kaldır.
        // Bu, aynı bakiyenin birden fazla kez talep edilmesini engeller.
        env.storage().instance().remove(&DataKey::Balance);
    }
}

// Bu yardımcı fonksiyon, kontratın daha önce başlatılıp başlatılmadığını kontrol eder.
// `env`: Kontrat ortamı.
// `-> bool`: Kontrat başlatılmışsa `true`, değilse `false` döner.
fn is_initialized(env: &Env) -> bool {
    // Kontratın örnek depolamasında `DataKey::Init` anahtarının var olup olmadığını kontrol eder.
    env.storage().instance().has(&DataKey::Init)
}

// `test` adında bir modül olduğunu bildirir.
mod test;