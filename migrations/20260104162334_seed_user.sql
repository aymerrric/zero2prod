-- Add migration script here
INSERT INTO users (user_id, username, password_hash)
VALUES('faa16c09-2e76-43d7-8f5c-e0f94eea8ab0','admin','$argon2id$v=19$m=19456,t=2,p=1$0j8xP8a+myUkAPvlGslJGQ$5YcYPVhhxILn4RxX/02EYubg3oqMVtIGDblAgSu/zpc');