// Bu satır, bir Rust özelliğidir (attribute) ve bu dosyanın içeriğinin yalnızca
// testler çalıştırıldığında (örneğin `cargo test` komutu ile) derlenmesini sağlar.
// Test olmayan yapılar (builds) için bu kod dahil edilmez.
#![cfg(test)]
// Bu satır, Rust standart kütüphanesini (std) bu test modülüne dahil eder.
// Ana kontrat kodumuz `#![no_std]` olsa da, test ortamları genellikle
// `std` kütüphanesinin sağladığı bazı araçlara ihtiyaç duyar (örn: yazdırma, test framework'ünün kendisi).
extern crate std;

// `super::*` ifadesi, bir üst modüldeki (bu durumda ana kontrat kodunuzun olduğu yer, muhtemelen lib.rs)
// tüm public öğeleri (struct'lar, enum'lar, fonksiyonlar vb.) bu modülün kapsamına dahil eder.
// Bu sayede `ClaimableBalanceContract`, `DataKey` gibi tanımları doğrudan kullanabiliriz.
use super::*;
// `soroban_sdk::testutils` modülünden test için gerekli araçları içe aktarır:
// - `Address as _`: `Address` tipi için `.generate()` gibi test yardımcı metotlarını sağlayan trait'leri içe aktarır.
// - `AuthorizedFunction`: Yetkilendirilmesi beklenen bir fonksiyon çağrısını temsil eder.
// - `AuthorizedInvocation`: Bir yetkilendirme çağrısının tüm detaylarını (fonksiyon, argümanlar, alt çağrılar) temsil eder.
// - `Ledger`: Test ortamındaki defter (ledger) durumunu (örn: zaman damgası) değiştirmek için kullanılır.
use soroban_sdk::testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation, Ledger};
// `soroban_sdk`'dan sık kullanılan diğer öğeleri içe aktarır:
// - `symbol_short`: Kısa semboller oluşturmak için bir makro (genellikle fonksiyon isimleri için kullanılır).
// - `token`: Token kontratlarıyla etkileşim için modül.
// - `vec`: Soroban ortamında dinamik vektör (liste) oluşturmak için bir makro.
// - `Address`: Bir Soroban adresini temsil eder.
// - `Env`: Kontratın çalıştığı Soroban ortamını temsil eder.
// - `IntoVal`: Rust tiplerini Soroban'ın temel `Val` tipine dönüştürmek için bir trait.
use soroban_sdk::{symbol_short, token, vec, Address, Env, IntoVal};
// `token` modülündeki `Client`'ı `TokenClient` olarak yeniden adlandırarak içe aktarır.
// Bu, bir token kontratıyla etkileşim kurmak için kullanılır.
use token::Client as TokenClient;
// `token` modülündeki `StellarAssetClient`'ı `TokenAdminClient` olarak yeniden adlandırarak içe aktarır.
// Bu, bir token kontratında yönetici işlemleri (örn: token basma) yapmak için kullanılır.
use token::StellarAssetClient as TokenAdminClient;

// Bu fonksiyon, test ortamında yeni bir token kontratı oluşturur ve
// hem normal kullanıcılar için bir istemci (TokenClient) hem de yönetici işlemleri için
// bir istemci (TokenAdminClient) döndürür.
// `e`: Soroban test ortamı.
// `admin`: Oluşturulacak token kontratının yöneticisi olacak adres.
fn create_token_contract<'a>(e: &Env, admin: &Address) -> (TokenClient<'a>, TokenAdminClient<'a>) {
    // `e.register_stellar_asset_contract_v2(admin.clone())` ile yeni bir standart Stellar varlık kontratı (token)
    // test ortamında kaydedilir (deploy edilir) ve `admin` adresi bu token'ın yöneticisi olarak atanır.
    // `sac` (Stellar Asset Contract), bu yeni deploy edilmiş token kontratının adresini içerir.
    let sac = e.register_stellar_asset_contract_v2(admin.clone());
    // Bir tuple içinde iki istemci döndürülür:
    (
        // Yeni oluşturulan token kontratı için bir `TokenClient`.
        token::Client::new(e, &sac.address()),
        // Aynı token kontratı için bir `TokenAdminClient`.
        token::StellarAssetClient::new(e, &sac.address()),
    )
}

// Bu fonksiyon, test ettiğimiz `ClaimableBalanceContract`'ı test ortamında oluşturur (kaydeder/deploy eder)
// ve bu kontratla etkileşim kurmak için bir istemci (ClaimableBalanceContractClient) döndürür.
// `ClaimableBalanceContractClient`, Soroban SDK tarafından kontratınızın public arayüzüne göre otomatik olarak oluşturulur.
fn create_claimable_balance_contract<'a>(e: &Env) -> ClaimableBalanceContractClient<'a> {
    // `ClaimableBalanceContractClient::new` ile yeni bir istemci oluşturulur.
    // İkinci argüman olarak `e.register(ClaimableBalanceContract, ())` çağrılır:
    //   - `ClaimableBalanceContract`: Deploy edilecek kontratımızın tipi.
    //   - `()`: Kontratımızın `#[contract]` struct'ı için başlangıç argümanları. Bizim kontratımızda
    //     `ClaimableBalanceContract` struct'ı boş olduğu için boş bir tuple (`()`) veriyoruz.
    // `e.register` fonksiyonu, kontratı deploy eder ve adresini döndürür.
    ClaimableBalanceContractClient::new(e, &e.register(ClaimableBalanceContract, ()))
}

// Bu struct, her test için ortak olan test ortamını ve bileşenlerini bir arada tutar.
// Test kurulumunu kolaylaştırır.
struct ClaimableBalanceTest<'a> {
    // Soroban test ortamı.
    env: Env,
    // Token'ları yatıracak olan adres.
    deposit_address: Address,
    // Bakiyeyi talep edebilecek adreslerden oluşan bir dizi (bu örnekte 3 tane).
    claim_addresses: [Address; 3],
    // Testlerde kullanılacak token kontratının istemcisi.
    token: TokenClient<'a>,
    // Test ettiğimiz `ClaimableBalanceContract`'ın istemcisi.
    contract: ClaimableBalanceContractClient<'a>,
}

// `ClaimableBalanceTest` struct'ı için metotlar.
impl<'a> ClaimableBalanceTest<'a> {
    // Bu `setup` fonksiyonu, bir test için gerekli tüm başlangıç ayarlarını yapar
    // ve bir `ClaimableBalanceTest` örneği döndürür.
    fn setup() -> Self {
        // Varsayılan bir Soroban test ortamı oluşturur.
        let env = Env::default();
        // `env.mock_all_auths()`: Ortamdaki tüm yetkilendirme (auth) çağrılarının
        // otomatik olarak "onaylanmış" gibi davranmasını sağlar VE hangi adresin hangi
        // fonksiyonu hangi argümanlarla çağırmak için yetkilendirme istediğini kaydeder.
        // Bu, `require_auth()` çağrılarının test edilmesini sağlar.
        env.mock_all_auths();

        // Defter (ledger) durumunu değiştirir.
        env.ledger().with_mut(|li| {
            // Defterin zaman damgasını `12345` olarak ayarlar. Bu, zaman kilitli testler için önemlidir.
            li.timestamp = 12345;
        });

        // `deposit_address` için rastgele bir Soroban adresi oluşturur.
        let deposit_address = Address::generate(&env);

        // `claim_addresses` dizisi için üç adet rastgele Soroban adresi oluşturur.
        let claim_addresses = [
            Address::generate(&env),
            Address::generate(&env),
            Address::generate(&env),
        ];

        // Token kontratının yöneticisi için rastgele bir adres oluşturur.
        let token_admin = Address::generate(&env);

        // `create_token_contract` helper fonksiyonunu kullanarak bir token kontratı oluşturur
        // ve token istemcisi ile token admin istemcisini alır.
        let (token, token_admin_client) = create_token_contract(&env, &token_admin);
        // Token admin istemcisini kullanarak `deposit_address`'e `1000` birim token basar (mint eder).
        // Bu, yatıran kişinin kontrata yatıracak token'lara sahip olmasını sağlar.
        token_admin_client.mint(&deposit_address, &1000);

        // `create_claimable_balance_contract` helper fonksiyonunu kullanarak
        // test ettiğimiz `ClaimableBalanceContract`'ı deploy eder ve istemcisini alır.
        let contract = create_claimable_balance_contract(&env);
        // Ayarlanan tüm bileşenlerle `ClaimableBalanceTest` struct'ını oluşturup döndürür.
        ClaimableBalanceTest {
            env,
            deposit_address,
            claim_addresses,
            token,
            contract,
        }
    }
}

// Bu, bir test fonksiyonudur. `#[test]` attribute'ü Rust'ın test çalıştırıcısının
// bu fonksiyonu bir test olarak tanımasını sağlar.
#[test]
fn test_deposit_and_claim() {
    // `ClaimableBalanceTest::setup()` ile test ortamını kurar.
    let test = ClaimableBalanceTest::setup();
    // Kurulan `contract` istemcisi üzerinden `deposit` fonksiyonunu çağırır:
    // - `deposit_address`: Yatırıcı.
    // - `token.address`: Yatırılacak token'ın adresi.
    // - `800`: Yatırılacak miktar.
    // - `vec![...]`: Talepçilerin listesi (bu durumda `claim_addresses[0]` ve `claim_addresses[1]`).
    // - `TimeBound`: Zaman sınırı (timestamp 12346'dan önce). Ledger timestamp'i 12345 olduğu için bu geçerli bir zaman.
    test.contract.deposit(
        &test.deposit_address,
        &test.token.address,
        &800,
        &vec![
            &test.env, // `vec!` makrosu için `Env` referansı gerekir.
            test.claim_addresses[0].clone(),
            test.claim_addresses[1].clone(),
        ],
        &TimeBound {
            kind: TimeBoundKind::Before,
            timestamp: 12346,
        },
    );

    // `test.env.auths()`: `deposit` çağrısı sırasında yapılan tüm yetkilendirme
    // taleplerinin bir kaydını döndürür.
    // Bu `assert_eq!` ile beklenen yetkilendirme çağrılarının yapılıp yapılmadığı kontrol edilir.
    // Beklenti:
    // 1. `deposit_address`, `deposit` fonksiyonunu belirtilen argümanlarla çağırmak için yetkilendirme yapmıştır.
    // 2. Bu `deposit` çağrısının içinde, `token.transfer` fonksiyonu çağrılmıştır ve
    //    `deposit_address`, kendi token'larını kontrata transfer etmek için bu alt çağrıyı da yetkilendirmiştir.
    assert_eq!(
        test.env.auths(), // Gerçekleşen yetkilendirmeler
        [( // Beklenen yetkilendirmeler (tek bir ana yetkilendirme var)
            test.deposit_address.clone(), // Yetki veren adres
            AuthorizedInvocation { // Yetkilendirilen çağrı
                function: AuthorizedFunction::Contract(( // Kontrat çağrısı
                    test.contract.address.clone(), // Hedef kontrat (ClaimableBalanceContract)
                    symbol_short!("deposit"),     // Çağrılan fonksiyon adı
                    // Fonksiyon argümanları (bir tuple olarak ve Soroban Val tipine dönüştürülmüş)
                    (
                        test.deposit_address.clone(),
                        test.token.address.clone(),
                        800_i128, // Miktar
                        vec![
                            &test.env,
                            test.claim_addresses[0].clone(),
                            test.claim_addresses[1].clone()
                        ],
                        TimeBound {
                            kind: TimeBoundKind::Before,
                            timestamp: 12346,
                        },
                    )
                        .into_val(&test.env),
                )),
                sub_invocations: std::vec![AuthorizedInvocation { // Alt çağrılar (burada token transferi)
                    function: AuthorizedFunction::Contract((
                        test.token.address.clone(),      // Hedef kontrat (Token kontratı)
                        symbol_short!("transfer"),       // Çağrılan fonksiyon
                        (
                            test.deposit_address.clone(), // Transferin kaynağı
                            &test.contract.address,       // Transferin hedefi (ClaimableBalanceContract)
                            800_i128,                     // Miktar
                        )
                            .into_val(&test.env),
                    )),
                    sub_invocations: std::vec![] // Token transferinin başka alt çağrısı yok
                }]
            }
        ),]
    );

    // Token bakiyelerini kontrol eder:
    // Yatırıcının bakiyesi 1000 - 800 = 200 olmalı.
    assert_eq!(test.token.balance(&test.deposit_address), 200);
    // Kontratın bakiyesi 800 olmalı.
    assert_eq!(test.token.balance(&test.contract.address), 800);
    // Henüz talep etmemiş olan `claim_addresses[1]`'in bakiyesi 0 olmalı.
    assert_eq!(test.token.balance(&test.claim_addresses[1]), 0);

    // `claim_addresses[1]` adresini kullanarak `claim` fonksiyonunu çağırır.
    test.contract.claim(&test.claim_addresses[1]);
    // `claim` çağrısı için yetkilendirme kontrolü:
    // Beklenti: `claim_addresses[1]`, `claim` fonksiyonunu kendisi argümanıyla çağırmak için yetkilendirme yapmıştır.
    // Token transferi kontrattan olduğu için `claim_addresses[1]`'in ek bir alt çağrı yetkilendirmesi gerekmez.
    assert_eq!(
        test.env.auths(),
        [(
            test.claim_addresses[1].clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    test.contract.address.clone(),
                    symbol_short!("claim"),
                    (test.claim_addresses[1].clone(),).into_val(&test.env), // Argüman: talep eden adres
                )),
                sub_invocations: std::vec![] // Beklenen alt çağrı yok
            }
        ),]
    );

    // Talep sonrası token bakiyelerini kontrol eder:
    // Yatırıcının bakiyesi değişmemeli (200).
    assert_eq!(test.token.balance(&test.deposit_address), 200);
    // Kontratın bakiyesi 0 olmalı (tüm tokenlar talep edildi).
    assert_eq!(test.token.balance(&test.contract.address), 0);
    // `claim_addresses[1]`'in bakiyesi 800 olmalı.
    assert_eq!(test.token.balance(&test.claim_addresses[1]), 800);
}

// Bu test, kontrata iki kez para yatırmanın mümkün olmadığını kontrol eder.
// `#[should_panic(...)]` attribute'ü, testin paniklemesini beklediğimizi ve
// panik mesajının `expected` string'ini içermesi gerektiğini belirtir.
#[test]
#[should_panic(expected = "contract has been already initialized")]
fn test_double_deposit_not_possible() {
    let test = ClaimableBalanceTest::setup();
    // İlk deposit
    test.contract.deposit(
        &test.deposit_address,
        &test.token.address,
        &1,
        &vec![&test.env, test.claim_addresses[0].clone()],
        &TimeBound {
            kind: TimeBoundKind::Before,
            timestamp: 12346,
        },
    );
    // İkinci deposit denemesi - bu paniklemeli.
    test.contract.deposit(
        &test.deposit_address,
        &test.token.address,
        &1,
        &vec![&test.env, test.claim_addresses[0].clone()],
        &TimeBound {
            kind: TimeBoundKind::Before,
            timestamp: 12346,
        },
    );
}

// Bu test, yetkisiz bir talepçinin bakiyeyi talep edemeyeceğini kontrol eder.
#[test]
#[should_panic(expected = "claimant is not allowed to claim this balance")]
fn test_unauthorized_claim_not_possible() {
    let test = ClaimableBalanceTest::setup();
    // Deposit yapılır, talepçiler `claim_addresses[0]` ve `claim_addresses[1]`.
    test.contract.deposit(
        &test.deposit_address,
        &test.token.address,
        &800,
        &vec![
            &test.env,
            test.claim_addresses[0].clone(),
            test.claim_addresses[1].clone(),
        ],
        &TimeBound {
            kind: TimeBoundKind::Before,
            timestamp: 12346,
        },
    );

    // `claim_addresses[2]` (yetkisiz talepçi) talep etmeye çalışır - bu paniklemeli.
    test.contract.claim(&test.claim_addresses[2]);
}

// Bu test, zaman sınırı karşılanmadığında bakiyenin talep edilemeyeceğini kontrol eder.
#[test]
#[should_panic(expected = "time predicate is not fulfilled")]
fn test_out_of_time_bound_claim_not_possible() {
    let test = ClaimableBalanceTest::setup();
    // Deposit yapılır. Zaman sınırı: timestamp `12346`'dan *sonra*.
    // Defterin mevcut zaman damgası `12345` (setup'ta ayarlandı).
    test.contract.deposit(
        &test.deposit_address,
        &test.token.address,
        &800,
        &vec![&test.env, test.claim_addresses[0].clone()],
        &TimeBound {
            kind: TimeBoundKind::After, // 'After' türü
            timestamp: 12346,          // 12346'dan sonra
        },
    );

    // `claim_addresses[0]` talep etmeye çalışır. Zaman `12345`, koşul `12346`'dan sonra, bu yüzden paniklemeli.
    test.contract.claim(&test.claim_addresses[0]);
}

// Bu test, aynı bakiyenin iki kez talep edilemeyeceğini kontrol eder.
// `#[should_panic]` (expected olmadan) sadece herhangi bir panik bekler.
// İlk `claim` başarılı olacak, ikinci `claim` ise `DataKey::Balance` depolamada bulunamadığı için
// `env.storage().instance().get(&DataKey::Balance).unwrap()` satırında panikleyecektir.
#[test]
#[should_panic]
fn test_double_claim_not_possible() {
    let test = ClaimableBalanceTest::setup();
    test.contract.deposit(
        &test.deposit_address,
        &test.token.address,
        &800,
        &vec![&test.env, test.claim_addresses[0].clone()],
        &TimeBound {
            kind: TimeBoundKind::Before,
            timestamp: 12346,
        },
    );

    // İlk talep (başarılı olmalı).
    test.contract.claim(&test.claim_addresses[0]);
    assert_eq!(test.token.balance(&test.claim_addresses[0]), 800);
    // İkinci talep denemesi (aynı kişi tarafından) - bu paniklemeli.
    test.contract.claim(&test.claim_addresses[0]);
}

// Bu test, bir bakiye talep edildikten sonra tekrar deposit yapılamayacağını kontrol eder.
// Kontrat `Init` olarak işaretlendiği için ikinci deposit başarısız olmalıdır.
#[test]
#[should_panic(expected = "contract has been already initialized")]
fn test_deposit_after_claim_not_possible() {
    let test = ClaimableBalanceTest::setup();
    // Deposit yapılır. Zaman sınırı: `12344`'ten sonra. Defter zamanı `12345`, yani hemen talep edilebilir.
    test.contract.deposit(
        &test.deposit_address,
        &test.token.address,
        &800,
        &vec![&test.env, test.claim_addresses[0].clone()],
        &TimeBound {
            kind: TimeBoundKind::After,
            timestamp: 12344,
        },
    );

    // Talep edilir.
    test.contract.claim(&test.claim_addresses[0]);
    assert_eq!(test.token.balance(&test.claim_addresses[0]), 800);
    // Talep edildikten sonra tekrar deposit yapmaya çalışılır - bu paniklemeli.
    test.contract.deposit(
        &test.deposit_address,
        &test.token.address,
        &200,
        &vec![&test.env, test.claim_addresses[0].clone()],
        &TimeBound {
            kind: TimeBoundKind::After,
            timestamp: 12344,
        },
    );
}