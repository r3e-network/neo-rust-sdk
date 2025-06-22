use criterion::{black_box, criterion_group, criterion_main, Criterion};
use neo3::prelude::*;

fn benchmark_script_builder_simple(c: &mut Criterion) {
    c.bench_function("script_builder_simple", |b| {
        b.iter(|| {
            let mut builder = ScriptBuilder::new();
            builder.push_integer(black_box(42));
            builder.push_data(black_box(b"test"));
            builder.emit(black_box(OpCode::ADD));
            let _script = builder.to_bytes();
        });
    });
}

fn benchmark_script_builder_contract_call(c: &mut Criterion) {
    let contract_hash = ScriptHash::from_str("0xd2a4cff31913016155e38e474a2c06d08be276cf").unwrap();
    
    c.bench_function("script_builder_contract_call", |b| {
        b.iter(|| {
            let mut builder = ScriptBuilder::new();
            builder.contract_call(
                black_box(&contract_hash),
                black_box("transfer"),
                black_box(&[
                    ContractParameter::h160(&contract_hash),
                    ContractParameter::h160(&contract_hash),
                    ContractParameter::integer(100),
                    ContractParameter::any(),
                ]),
                None,
            ).unwrap();
            let _script = builder.to_bytes();
        });
    });
}

fn benchmark_transaction_builder(c: &mut Criterion) {
    let account = Account::create().unwrap();
    
    c.bench_function("transaction_builder", |b| {
        b.iter(|| {
            let mut builder = TransactionBuilder::new();
            builder.set_script(black_box(vec![0x00, 0x01, 0x02]));
            builder.set_signers(vec![
                Signer::account(black_box(account.get_script_hash()))
                    .set_allowed_contracts(vec![])
                    .set_allowed_groups(vec![])
                    .set_rules(vec![]),
            ]);
            builder.set_nonce(black_box(12345));
            builder.set_system_fee(black_box(1000000));
            builder.set_network_fee(black_box(500000));
            builder.set_valid_until_block(black_box(1000000));
            let _tx = builder.build();
        });
    });
}

criterion_group!(
    benches,
    benchmark_script_builder_simple,
    benchmark_script_builder_contract_call,
    benchmark_transaction_builder
);
criterion_main!(benches);