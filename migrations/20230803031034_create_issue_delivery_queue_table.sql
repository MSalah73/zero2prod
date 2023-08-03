CREATE TABLE issue_delivery_queue (
	newsletter_issue_id uuid NOT NULL
		REFERENCES newsletter_issues (newsletter_issue_id),
	subscriber_email TEXT NOT NULL,
	PRiMARY KEY(newsletter_issue_id,subscriber_email)
);
