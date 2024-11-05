from django.conf import settings
from django.db import models
from django.utils.translation import gettext_lazy


class AbstractModel(models.Model):
    created_by = models.ForeignKey(settings.AUTH_USER_MODEL, on_delete=models.PROTECT)
    created_at = models.DateTimeField(auto_now_add=True)
    updated_by = models.ForeignKey(
        settings.AUTH_USER_MODEL, on_delete=models.PROTECT, related_name="+"
    )
    updated_at = models.DateTimeField(auto_now=True)

    class Meta:
        abstract = True


class Level(AbstractModel):
    name = models.CharField(max_length=200)
    description = models.TextField()

    def __str__(self):
        return self.name


class Subject(AbstractModel):
    name = models.CharField(max_length=200)
    description = models.TextField()
    level = models.ForeignKey(Level, on_delete=models.PROTECT)

    def __str__(self):
        return self.name


class Category(AbstractModel):
    name = models.CharField(max_length=200)
    description = models.TextField()
    subject = models.ForeignKey(Subject, on_delete=models.PROTECT)

    def __str__(self):
        return self.name


class Topic(AbstractModel):
    name = models.CharField(max_length=200)
    description = models.TextField()
    category = models.ForeignKey(Category, on_delete=models.PROTECT)

    def __str__(self):
        return self.name


# TODO: Update QuestionGroup's updated_by and updated_date whenever its child Question / Choice gets updated.
class QuestionGroup(AbstractModel):
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


class Question(models.Model):
    question_group = models.ForeignKey(QuestionGroup, on_delete=models.PROTECT)
    question_text = models.TextField()
    question_image = models.ImageField(null=True, blank=True)
    solution_description = models.TextField()
    marks = models.IntegerField()
    time_limit = models.DurationField()

    solution_text = models.TextField()
    solution_value = models.DecimalField(max_digits=30, decimal_places=8)

    class Type(models.TextChoices):
        MULTIPLE_CHOICE = "MC", gettext_lazy("Multiple Choice")
        TEXT_ANSWER = "TXT", gettext_lazy("Text Answer")
        NUMERICAL = "NUM", gettext_lazy("Numerical")

    type = models.CharField(
        max_length=3, choices=Type.choices, default=Type.MULTIPLE_CHOICE
    )

    def __str__(self):
        return self.question_text


class Choice(models.Model):
    multi_choice_question = models.ForeignKey(
        Question, on_delete=models.PROTECT, related_name="choices"
    )
    choice_text = models.CharField(max_length=200)
    choice_image = models.ImageField(null=True, blank=True)
    is_correct = models.BooleanField(default=False)

    def __str__(self):
        return self.choice_text
