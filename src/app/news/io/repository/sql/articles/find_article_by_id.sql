SELECT
    a.id,
    a.space_id,
    a.author_id,
    COALESCE(u.coach_name, '') AS author_name,
    a.title,
    a.abstract,
    a.tags,
    a.image,
    a.content,
    a.created_at
FROM articles a
LEFT JOIN auth__users u ON u.id = a.author_id
WHERE a.id = $1