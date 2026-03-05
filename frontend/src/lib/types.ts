/**
 * Full user object returned by the API (e.g. /api/users/me, /api/auth/login).
 *
 * This is a superset of SessionUser — it includes profile fields such as
 * `bio` and `interest_category_ids` that are not stored in the auth session.
 */
export type UserResponse = {
  id: string;
  username: string;
  email?: string | null;
  bio?: string;
  is_premium: boolean;
  is_age_verified: boolean;
  created_at: string;
  interest_category_ids?: string[];
  deleted_at?: string | null;
  deletion_scheduled_at?: string | null;
};

/** Response from login, register, and OAuth exchange endpoints. */
export type AuthResponse = {
  token: string;
  user: UserResponse;
};
