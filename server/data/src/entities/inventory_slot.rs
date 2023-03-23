//! `SeaORM` Entity. Generated by sea-orm-codegen 0.10.6

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "inventory_slot")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub inv_type: i32,
    pub slot: i32,
    pub char_id: i32,
    pub equip_item_id: Option<i32>,
    pub stack_item_id: Option<i32>,
    pub pet_item_id: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::character::Entity",
        from = "Column::CharId",
        to = "super::character::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    Character,
    #[sea_orm(
        belongs_to = "super::equip_item::Entity",
        from = "Column::EquipItemId",
        to = "super::equip_item::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    EquipItem,
    #[sea_orm(
        belongs_to = "super::item_stack::Entity",
        from = "Column::StackItemId",
        to = "super::item_stack::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    ItemStack,
    #[sea_orm(
        belongs_to = "super::pet_item::Entity",
        from = "Column::PetItemId",
        to = "super::pet_item::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    PetItem,
}

impl Related<super::character::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Character.def()
    }
}

impl Related<super::equip_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::EquipItem.def()
    }
}

impl Related<super::item_stack::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ItemStack.def()
    }
}

impl Related<super::pet_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PetItem.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
