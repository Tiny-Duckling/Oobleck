from rest_framework import serializers

from .models import Level, Subject, TopicGroup, Topic, Question, Choice


class LevelSerializer(serializers.ModelSerializer):
    class Meta:
        model = Level
        fields = ('id', 'name', 'description')


class SubjectSerializer(serializers.ModelSerializer):
    class Meta:
        model = Subject
        fields = ('id', 'name', 'description', 'level')


class TopicGroupSerializer(serializers.ModelSerializer):
    class Meta:
        model = TopicGroup
        fields = ('id', 'name', 'description', 'subject', 'topic')


class TopicSerializer(serializers.ModelSerializer):
    class Meta:
        model = Topic
        fields = ('id', 'name', 'description', 'subject')


class QuestionSerializer(serializers.ModelSerializer):
    class Meta:
        model = Question
        fields = ('id', 'question_text', 'topic')


class ChoiceSerializer(serializers.ModelSerializer):
    class Meta:
        model = Choice
        fields = ('id', 'question', 'choice_text', 'is_correct')
