INSERT INTO categories (name, slug, description, sort_order)
VALUES
    ('Casual', 'casual', 'General random chat', 1),
    ('Romance', 'romance', 'Relationship and dating discussions', 2),
    ('Philosophy', 'philosophy', 'Meaning, ethics, and ideas', 3),
    ('Finance', 'finance', 'Money and markets', 4),
    ('Gaming', 'gaming', 'Games and game culture', 5),
    ('Music', 'music', 'Artists, songs, instruments', 6),
    ('Tech', 'tech', 'Programming and technology', 7),
    ('Politics', 'politics', 'Political discussion', 8)
ON CONFLICT (slug) DO NOTHING;

INSERT INTO languages (code, name)
VALUES
    ('en', 'English'),
    ('es', 'Spanish'),
    ('fr', 'French'),
    ('pt', 'Portuguese'),
    ('de', 'German')
ON CONFLICT (code) DO NOTHING;
