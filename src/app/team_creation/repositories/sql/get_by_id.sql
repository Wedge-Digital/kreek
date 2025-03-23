SELECT entity_id,
       created_by,
       coached_by,
       serialized,
       created_at,
       updated_at
from team_creation__draft_team
where entity_id = $1;