use diesel::Insertable;

#[derive(Insertable)]
#[diesel(table_name = crate::schema::languages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct InsertLanguage {
    pub english_name: String,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::word_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct InsertWordType {
    pub english_name: String,
}
