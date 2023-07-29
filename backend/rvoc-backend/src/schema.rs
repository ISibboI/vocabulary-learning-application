// @generated automatically by Diesel CLI.

diesel::table! {
    /// Representation of the `languages` table.
    ///
    /// (Automatically generated by Diesel.)
    languages (id) {
        /// The `id` column of the `languages` table.
        ///
        /// Its SQL type is `Int4`.
        ///
        /// (Automatically generated by Diesel.)
        id -> Int4,
        /// The `english_name` column of the `languages` table.
        ///
        /// Its SQL type is `Text`.
        ///
        /// (Automatically generated by Diesel.)
        english_name -> Text,
    }
}