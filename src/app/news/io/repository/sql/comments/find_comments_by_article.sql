SELECT
    c.id,
    c.article_id,
    c.author_id,
    COALESCE(u.coach_name, '') AS author_name,
    c.content,
    c.created_at
FROM comments c
LEFT JOIN auth__users u ON u.id = c.author_id
WHERE c.article_id = $1
ORDER BY c.created_at ASC