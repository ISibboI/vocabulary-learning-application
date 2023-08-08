// @generated automatically by Diesel CLI.

diesel::table! {
    /// Representation of the `job_queue` table.
    ///
    /// (Automatically generated by Diesel.)
    job_queue (scheduled_execution_time) {
        /// The `scheduled_execution_time` column of the `job_queue` table.
        ///
        /// Its SQL type is `Timestamp`.
        ///
        /// (Automatically generated by Diesel.)
        scheduled_execution_time -> Timestamp,
        /// The `name` column of the `job_queue` table.
        ///
        /// Its SQL type is `Text`.
        ///
        /// (Automatically generated by Diesel.)
        name -> Text,
    }
}

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

diesel::table! {
    /// Representation of the `word_types` table.
    ///
    /// (Automatically generated by Diesel.)
    word_types (id) {
        /// The `id` column of the `word_types` table.
        ///
        /// Its SQL type is `Int4`.
        ///
        /// (Automatically generated by Diesel.)
        id -> Int4,
        /// The `english_name` column of the `word_types` table.
        ///
        /// Its SQL type is `Text`.
        ///
        /// (Automatically generated by Diesel.)
        english_name -> Text,
    }
}

diesel::table! {
    /// Representation of the `words` table.
    ///
    /// (Automatically generated by Diesel.)
    words (word, word_type, language) {
        /// The `word` column of the `words` table.
        ///
        /// Its SQL type is `Text`.
        ///
        /// (Automatically generated by Diesel.)
        word -> Text,
        /// The `word_type` column of the `words` table.
        ///
        /// Its SQL type is `Int4`.
        ///
        /// (Automatically generated by Diesel.)
        word_type -> Int4,
        /// The `language` column of the `words` table.
        ///
        /// Its SQL type is `Int4`.
        ///
        /// (Automatically generated by Diesel.)
        language -> Int4,
    }
}

diesel::joinable!(words -> languages (language));
diesel::joinable!(words -> word_types (word_type));

diesel::allow_tables_to_appear_in_same_query!(
    job_queue,
    languages,
    word_types,
    words,
);
