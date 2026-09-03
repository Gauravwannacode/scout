-- Where an opening physically happens, so "hackathons near me" is answerable.
--
-- Devpost reports everything as "Online", but Devfolio and Unstop both carry
-- real cities — which is why local events are findable at all, and why this
-- column matters more for the Indian platforms than the global ones.
ALTER TABLE item ADD COLUMN location TEXT;

-- NULL means the source did not say, which is different from "not online".
ALTER TABLE item ADD COLUMN is_online INTEGER;

CREATE INDEX IF NOT EXISTS item_location ON item (location);
