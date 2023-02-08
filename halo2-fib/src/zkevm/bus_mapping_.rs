 use bus_mapping::{Error, mock::BlockData};
 use bus_mapping::state_db::{self, StateDB, CodeDB};
 use eth_types::{
     self, address, Address, Word, Hash, U64, GethExecTrace, GethExecStep, geth_types::GethData, bytecode
 };
 use mock::test_ctx::{TestContext, helpers::*};
 use eth_types::evm_types::Gas;
 use bus_mapping::circuit_input_builder::{Block, CircuitInputBuilder};
 use serde_json;

#[test]
pub fn test() {
    let input_trace = r#"
 [
     {
         "pc": 5,
         "op": "PUSH1",
         "gas": 82,
         "gasCost": 3,
         "depth": 1,
         "stack": [],
         "memory": [
           "0000000000000000000000000000000000000000000000000000000000000000",
           "0000000000000000000000000000000000000000000000000000000000000000",
           "0000000000000000000000000000000000000000000000000000000000000080"
         ]
       },
       {
         "pc": 7,
         "op": "MLOAD",
         "gas": 79,
         "gasCost": 3,
         "depth": 1,
         "stack": [
           "40"
         ],
         "memory": [
           "0000000000000000000000000000000000000000000000000000000000000000",
           "0000000000000000000000000000000000000000000000000000000000000000",
           "0000000000000000000000000000000000000000000000000000000000000080"
         ]
       },
       {
         "pc": 8,
         "op": "STOP",
         "gas": 76,
         "gasCost": 0,
         "depth": 1,
         "stack": [
           "80"
         ],
         "memory": [
           "0000000000000000000000000000000000000000000000000000000000000000",
           "0000000000000000000000000000000000000000000000000000000000000000",
           "0000000000000000000000000000000000000000000000000000000000000080"
         ]
       }
 ]
 "#;

    let code = bytecode!{
        // Write 0x6f to storage slot 0
        PUSH1(0x6fu64)
        PUSH1(0x00u64)
        SSTORE
        // Load storage slot 0
        PUSH1(0x00u64)
        SLOAD
        STOP
    };

    let block: GethData = TestContext::<2,1>::new(
        None,
        account_0_code_account_1_no_code(code),
        tx_from_1_to_0,
        |block, _tx| block.number(0xcafeu64)
    ).unwrap().into();

    let mut builder = BlockData::new_from_geth_data(block.clone()).new_circuit_input_builder();
    builder
        .handle_block(&block.eth_block, &block.geth_traces)
        .unwrap();

    let geth_steps: Vec<GethExecStep> = serde_json::from_str(input_trace).unwrap();
    let geth_trace = GethExecTrace {
        return_value: "".to_string(),
        gas: Gas(block.eth_block.transactions[0].gas.as_u64()),
        failed: false,
        struct_logs: geth_steps,
    };
    // Get an ordered vector with all of the Stack operations of this trace.
    let stack_ops = builder.block.container.sorted_stack();
    // You can also iterate over the steps of the trace and witness the EVM Proof.
    builder.block.txs()[0].steps().iter();

}
