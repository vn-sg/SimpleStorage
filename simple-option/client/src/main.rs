use serde::Deserialize;
use serde::Serialize;
use serde::Serializer;
use serde::ser::SerializeStruct;
use serde_json;
use serde_json::Value;
use std::env;
use std::env::args;
use cosmwasm_std::{
    to_binary, Binary, Api, ExternalApi, ExternalQuerier, Querier
};


#[derive(Serialize, Deserialize)]
pub struct Person {
    name: String,
    age: u8,
}

#[derive(Serialize, Deserialize)]
pub struct Building {
    name: String,
    content: Value,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Register { name: String },
    Transfer { name: String, to: String },
    RegisterTrustBoost { detail: RegisterTrustBoostDetail, signature: String},
    RegisterTrustBoost2 { name: String, user: String, signature: String},

}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct RegisterTrustBoostDetail {
    name: String, 
    user: String
}

pub struct SkipSerialized {
    val: Vec<u8>
}

// impl Serialize for SkipSerialized {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer {
//             Ok(Serializer::Ok)
//     }
// }


pub struct MyStruct {
    pub value: String,
}

impl Serialize for MyStruct {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer {
            let mut state = serializer.serialize_struct("Color", 3)?;
            state.serialize_field("r", "testR")?;
            state.serialize_field("g", "testG")?;
            state.serialize_field("b", "testB")?;
            state.end()
    }
}

fn main () {
    let args: Vec<String> = env::args().collect();
    let param = &args[1];
    println!("First Param {}", param);
    let ex = ExternalApi::new();
    //ex.addr_validate(human);
    let querier = ExternalQuerier::new();
    querier.raw_query(bin_request)

}

fn main2() {

    let args: Vec<String> = env::args().collect();
    let param = &args[1];
    println!("First Param {}", param);

    let contract_exec = ExecuteMsg::Register { name: param.to_string() };
    let binary_result = to_binary(&contract_exec).unwrap();
    let mut vec_u8 = binary_result.0.to_vec();
    println!("{:?}", vec_u8);
    println!("base64 encoded {}", binary_result);

    let test = MyStruct{
        value: "test".to_string()
    };
    let test = vec_u8.to_vec();
    let serialized = serde_json::to_string(&test).unwrap();
    let serialized_bytes = serialized.as_bytes();
    println!("serialized {}", serialized);
    print!("serialized bytes: ");
    for i in serialized_bytes.iter() {
        print!("{},", *i as u8)
    }
    
    println!();

    println!("Charified vec_u8");
    for i in vec_u8.iter_mut() {
        print!("{}", *i as char)
    }

    //eyJyZWdpc3RlciI6eyJuYW1lIjoidGVzdF9mcm9tX2NsaWVudCJ9fQ==

    println!("Reverse_binary");

    let reverse_binary = binary_result.0.to_vec();
    let reverse_binary = Binary(reverse_binary);

    println!("reverse_binary {}", reverse_binary);
    
}


/* 
fn main() {
    let exec_msg = ExecuteMsg::Register { name: "test_exec".to_string() };
    let exec_msg_str = exec_msg.serialize(serializer);
    //println!("Serde exec_msg_str json_string to_string {}", exec_msg_str);


    let args: Vec<String> = env::args().collect();

    let param = &args[1];

    println!("Hello World! first param {}", param);

    let first_person = Person {
        name: "koala".to_string(),
        age: 15,
    };

    let json_string = serde_json::to_string(&first_person).unwrap();


    println!("Serde json_string to_string {}", json_string);

    // https://docs.rs/serde_json/latest/serde_json/
    // '{"name":"koala","age":15}}'
    // "{\"Person\": {\"name\":\"koala\",\"age\":15}}" << DOESNT WORK 
    let target = param.as_str();
    let target =  "{\"name\":\"koala\",\"age\":34}"; // OK
    //let target =  "{\\\"name\\\":\\\"koala\\\",\\\"age\\\":34}";
    println!("TARGET {}", target);
    let person_from_string: Result<Person,_> = serde_json::from_str(target);
    // let person_from_string: Result<Person,_> = serde_json::from_str(json_string.as_str());
    let person_from_string = person_from_string.unwrap();

    println!("Serde json_string from_string name {} age {}", person_from_string.name, person_from_string.age);

    let tempo: Value = serde_json::from_str(target).unwrap();
    let tempo2: String = serde_json::to_string(&tempo).unwrap();
    println!("Serde stringified {}", tempo2);

    let target = "{\"name\":\"koala_building\",\"content\": {\"name\": \"koala_person\",\"age\": 50}}";
    let building_from_string: Result<Building,_> = serde_json::from_str(target);
    let building_from_string = building_from_string.unwrap();
    println!("Building parsed name: {} content: {}", building_from_string.name, building_from_string.content["age"]);

    let person_from_string: Result<Person,_> = serde_json::from_str(&building_from_string.content.to_string());
    let person_from_string = person_from_string.unwrap();
    println!("Serde json_string from_string Person's building name {} age {}", person_from_string.name, person_from_string.age);

}
*/