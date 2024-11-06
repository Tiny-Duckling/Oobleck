# TODO: Update QuestionGroup's updated_by and updated_date whenever its child Question / Choice gets updated.
from django.conf import settings
from django.db import models

from .topic import Topic
from .create_update_model import CreateUpdateModel


class QuestionGroup(CreateUpdateModel):
    topics = models.ManyToManyField(Topic)
    passage_text = models.TextField()
    passage_image = models.ImageField(null=True, blank=True)

    is_approved = models.BooleanField(default=False)
    approved_at = models.DateTimeField(null=True, blank=True)
    approved_by = models.ForeignKey(
        settings.AUTH_USER_MODEL,
        on_delete=models.PROTECT,
        related_name="approved_question_groups",
        null=True,
        blank=True,
    )

    is_deleted = models.BooleanField(default=False)

    def __str__(self):
        return self.passage_text
