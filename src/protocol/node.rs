use std::io::{Read, Write};

use anyhow::{bail, Context};
use byteorder::{ReadBytesExt, WriteBytesExt};

use crate::ident::Ident;
use crate::protocol::{BoundedString, Decode, Encode, VarInt};

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub children: Vec<VarInt>,
    pub data: NodeData,
    pub is_executable: bool,
    pub redirect_node: Option<VarInt>,
}

impl Node {
    pub fn mut_name(&mut self) -> &mut BoundedString<0, 32767> {
        match &mut self.data {
            NodeData::Root => {panic!("Can't set name for root node")}
            NodeData::Literal(literal) => {&mut literal.name}
            NodeData::Argument(argument) => {&mut argument.name}
        }
    }
}

impl Encode for Node {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        let enum_id = match self.data {
            NodeData::Root => 0,
            NodeData::Literal(_) => 1,
            NodeData::Argument(_) => 2,
        };

        let flags = enum_id
            | (self.is_executable as u8 * 0x04)
            | (self.redirect_node.is_some() as u8 * 0x08)
            | (if let NodeData::Argument(argument) = &self.data {
                argument.suggestions_type.is_some()
            } else {
                false
            } as u8
                * 0x10);

        w.write_u8(flags)?;
        self.children.encode(w)?;

        if let Some(redirect_node) = self.redirect_node {
            redirect_node.encode(w)?;
        }

        match &self.data {
            NodeData::Root => {}
            NodeData::Literal(literal) => literal.name.encode(w)?,
            NodeData::Argument(argument) => {
                argument.name.encode(w)?;
                argument.parser.encode(w)?;
                if let Some(suggestions_type) = &argument.suggestions_type {
                    suggestions_type.encode(w)?;
                }
            }
        }

        Ok(())
    }
}

impl Decode for Node {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let flags = r.read_u8()?;

        let is_executable = flags & 0x04 != 0;
        let redirect_node = if flags & 0x08 != 0 {
            Decode::decode(r)?
        } else {
            None
        };

        let children = Decode::decode(r)?;

        let enum_id = flags & 0x03;
        let data = match enum_id {
            0 => NodeData::Root,
            1 => NodeData::Literal(Literal {
                name: Decode::decode(r)?,
            }),
            2 => NodeData::Argument(Argument {
                name: Decode::decode(r)?,
                parser: Decode::decode(r)?,
                suggestions_type: if flags & 0x10 != 0 {
                    Decode::decode(r)?
                } else {
                    None
                },
            }),
            _ => bail!("Invalid NodeData variant"),
        };

        Ok(Node {
            children,
            data,
            is_executable,
            redirect_node,
        })
    }
}

#[derive(Clone, Debug, PartialEq, derive_more::From)]
pub enum NodeData {
    Root,
    Literal(Literal),
    Argument(Argument),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Literal {
    pub name: BoundedString<0, 32767>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Argument {
    pub name: BoundedString<0, 32767>,
    pub parser: Parser,
    pub suggestions_type: Option<Ident>,
}

def_enum! {
    #[derive(derive_more::From, PartialEq)]
    Parser: VarInt {
        BrigadierBool: bool = 0,
        BrigadierFloat: BrigadierFloat = 1,
        //BrigadierDouble: BrigadierDouble = 2,
        BrigadierInteger: BrigadierInteger = 3,
        BrigadierLong: BrigadierLong = 4,
        //TODO
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrigadierFloat {
    pub min: Option<f32>,
    pub max: Option<f32>,
}

impl Encode for BrigadierFloat {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        let flags = (self.min.is_some() as u8) << 0 | (self.max.is_some() as u8) << 1;
        w.write_u8(flags)?;
        if let Some(min) = self.min {
            min.encode(w)?;
        }
        if let Some(max) = self.max {
            max.encode(w)?;
        }
        Ok(())
    }
}

impl Decode for BrigadierFloat {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let flags = r.read_u8()?;
        let min = if flags & 0x01 != 0 {
            Decode::decode(r)?
        } else {
            None
        };
        let max = if flags & 0x02 != 0 {
            Decode::decode(r)?
        } else {
            None
        };
        Ok(Self { min, max })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrigadierInteger {
    pub min: Option<i32>,
    pub max: Option<i32>,
}

impl Encode for BrigadierInteger {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        let flags = (self.min.is_some() as u8) << 0 | (self.max.is_some() as u8) << 1;
        w.write_u8(flags)?;
        if let Some(min) = self.min {
            min.encode(w)?;
        }
        if let Some(max) = self.max {
            max.encode(w)?;
        }
        Ok(())
    }
}

impl Decode for BrigadierInteger {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let flags = r.read_u8()?;
        let min = if flags & 0x01 != 0 {
            Decode::decode(r)?
        } else {
            None
        };
        let max = if flags & 0x02 != 0 {
            Decode::decode(r)?
        } else {
            None
        };
        Ok(Self { min, max })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrigadierLong {
    pub min: Option<i64>,
    pub max: Option<i64>,
}

impl Encode for BrigadierLong {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        let flags = (self.min.is_some() as u8) << 0 | (self.max.is_some() as u8) << 1;
        w.write_u8(flags)?;
        if let Some(min) = self.min {
            min.encode(w)?;
        }
        if let Some(max) = self.max {
            max.encode(w)?;
        }
        Ok(())
    }
}

impl Decode for BrigadierLong {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let flags = r.read_u8()?;
        let min = if flags & 0x01 != 0 {
            Decode::decode(r)?
        } else {
            None
        };
        let max = if flags & 0x02 != 0 {
            Decode::decode(r)?
        } else {
            None
        };
        Ok(Self { min, max })
    }
}
