INSERT INTO flare_items (name, description, item_type, price_sparks, rarity, asset_data)
VALUES
    ('Sunset Username', 'Warm coral username color', 'username_color', 500, 'common', '{"color":"#ff6b35"}'),
    ('Midnight Bubble', 'Dark bubble style', 'bubble_theme', 900, 'rare', '{"theme":"midnight"}'),
    ('Golden Border', 'Profile golden border', 'profile_border', 1500, 'rare', '{"border":"gold"}'),
    ('Founder Badge', 'Early supporter badge', 'badge', 2500, 'legendary', '{"badge":"founder"}')
ON CONFLICT DO NOTHING;
