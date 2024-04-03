-- Add migration script here

DROP TABLE IF EXISTS res_info;
CREATE TABLE res_info (
    info_hash VARCHAR(100) PRIMARY KEY,
    res_type INT NOT NULL,
    create_time VARCHAR(100) NOT NULL,
    mod_time VARCHAR(100) NOT NULL,
    is_indexed INT NOT NULL
);

CREATE TABLE res_file (
    info_hash VARCHAR(100) NOT NULL,
    file_path VARCHAR(1000) NOT NULL,
    file_size INT NOT NULL,
    create_time VARCHAR(100) NOT NULL,
    mod_time VARCHAR(100) NOT NULL,
);

